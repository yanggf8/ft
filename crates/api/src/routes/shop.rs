//! 開運小物 v1「我有興趣」登記（spec docs/superpowers/specs/2026-11-22-shop-interest-design.md §4）。
//!
//! - 身分：登入可選（[`auth_user_opt`]，無效／過期 token 視同匿名，不回 401）＋瀏覽器
//!   `anonId`（localStorage 隨機 UUID；長度 8–64、`[A-Za-z0-9-]`，不合 400 `INVALID_ANON_ID`）。
//!   兩者皆無 → 400 `NO_IDENTITY`。不記 IP／UA（IP 只用於限流 key，不落庫）。
//! - `POST /api/shop/interest`：body cap → 限流 `shop:ip:{ip}` 30／小時（fail-open，無成本端點）
//!   → `itemId` 不在 CATALOG 400 `INVALID_ITEM` → 去重（`item_id` 且 user／anon 任一相符 →
//!   `already:true`；既有列無 user_id 而本次有登入 → `UPDATE` 補上，匿名轉登入不重算）→ 插一列。
//! - `GET /api/shop/interest/me?anonId=`：登入者 ∪ anonId 的品項，去重、只留目前 CATALOG（依目錄順序）。
//! - `DELETE /api/shop/interest/:itemId`（body `{"anonId"}` 可選，itemId 以路徑為準）：刪符合身分的列。
//! - DB 錯誤一律 500 `"db error"`，不回細節。
//! - 紅線：小物資料永不寫入 predictions／prediction_feedback／f8_assignments；不 import `symbolic`。

use super::super::{
    error,
    services::{clock, db, uuid},
};
use super::common::{auth_user_opt, body_too_large, client_ip, ok_json, rate_limit};
use super::R;
use ft_schema::api::{ShopInterestMeResponse, ShopInterestRequest, ShopInterestResponse};
use ft_schema::shop::{find, CATALOG, SHOP_CATALOG_VERSION};

use serde::Deserialize;
use worker::*;

/// 每 IP 每小時 30 次（POST）。
const IP_LIMIT: u32 = 30;
const IP_WINDOW_MS: f64 = 3_600_000.0;

/// `anonId` 格式：長度 8–64、只含 `[A-Za-z0-9-]`。
fn valid_anon_id(s: &str) -> bool {
    (8..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// 缺省允許（`None`）；有值但格式不合 → `Err`（→ 400 `INVALID_ANON_ID`）。
fn check_anon(anon: Option<&str>) -> std::result::Result<Option<&str>, ()> {
    match anon {
        None => Ok(None),
        Some(a) if valid_anon_id(a) => Ok(Some(a)),
        Some(_) => Err(()),
    }
}

#[derive(Debug, Deserialize)]
struct MatchRow {
    id: String,
    user_id: Option<String>,
}

/// 去重命中後要補 user_id 的列：本次有登入、且命中列裡沒有任何一列已屬於此 user 時，
/// 取第一個 user_id 為 NULL 的列（匿名轉登入）。已有本人列就不動，避免同 user 兩列。
fn upgrade_target<'a>(rows: &'a [MatchRow], user: Option<&str>) -> Option<&'a str> {
    let uid = user?;
    if rows.iter().any(|r| r.user_id.as_deref() == Some(uid)) {
        return None;
    }
    rows.iter()
        .find(|r| r.user_id.is_none())
        .map(|r| r.id.as_str())
}

/// 去重命中是否算「此身分已登記」：匿名時任一命中即是；登入時須有本人列或可補的匿名列——
/// 共用瀏覽器下 anonId 只命中**別人**的列時，本人仍要新增一列。
fn covered(rows: &[MatchRow], user: Option<&str>) -> bool {
    match user {
        None => !rows.is_empty(),
        Some(uid) => rows
            .iter()
            .any(|r| r.user_id.is_none() || r.user_id.as_deref() == Some(uid)),
    }
}

#[derive(Debug, Deserialize)]
struct ItemRow {
    item_id: Option<String>,
}

/// 聯集結果 → 去重、只留目前 CATALOG 內的 id，依目錄順序。
fn me_items(rows: &[ItemRow]) -> Vec<String> {
    CATALOG
        .iter()
        .filter(|it| rows.iter().any(|r| r.item_id.as_deref() == Some(it.id)))
        .map(|it| it.id.to_string())
        .collect()
}

/// DELETE body：只取 `anonId`（itemId 以路徑為準）。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteBody {
    #[serde(default)]
    anon_id: Option<String>,
}

fn json<T: serde::Serialize>(v: &T) -> Response {
    let mut res = ok_json(
        &serde_json::to_value(v).unwrap_or(serde_json::Value::Null),
        200,
    );
    let _ = res.headers_mut().set("Cache-Control", "private, no-store");
    res
}

fn db_error() -> Response {
    error::error_code("db error", "DB_ERROR", 500)
}

fn payload_too_large() -> Response {
    error::error_code("payload too large", "PAYLOAD_TOO_LARGE", 413)
}

fn invalid_item() -> Response {
    error::error_code("Invalid item", "INVALID_ITEM", 400)
}

fn invalid_anon() -> Response {
    error::error_code("Invalid anonId", "INVALID_ANON_ID", 400)
}

fn no_identity() -> Response {
    error::error_code("Login or anonId required", "NO_IDENTITY", 400)
}

pub fn register(router: R<'static>) -> R<'static> {
    router
        .post_async("/api/shop/interest", |mut req, ctx| async move {
            if body_too_large(&req) {
                return Ok(payload_too_large());
            }
            if !rate_limit(
                &ctx,
                &format!("shop:ip:{}", client_ip(&req)),
                IP_LIMIT,
                IP_WINDOW_MS,
            )
            .await
            {
                return Ok(error::error_code("Too many requests", "RATE_LIMITED", 429));
            }
            let body: ShopInterestRequest = match req.json().await {
                Ok(v) => v,
                Err(_) => return Ok(error::error_code("Invalid JSON", "INVALID_JSON", 400)),
            };
            let Some(item) = find(&body.item_id) else {
                return Ok(invalid_item());
            };
            let Ok(anon) = check_anon(body.anon_id.as_deref()) else {
                return Ok(invalid_anon());
            };
            let user = auth_user_opt(&req, &ctx).await;
            if user.is_none() && anon.is_none() {
                return Ok(no_identity());
            }

            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(db_error()),
            };
            // NULL 參數的等式永不為真 → 只有帶值的條件生效
            let rows: Vec<MatchRow> = match db::all(
                &db,
                "SELECT id, user_id FROM shop_interest \
                 WHERE item_id=?1 AND (user_id=?2 OR anon_id=?3) \
                 ORDER BY created_at",
                &[
                    &db::text(item.id),
                    &db::opt_text(user.as_deref()),
                    &db::opt_text(anon),
                ],
            )
            .await
            {
                Ok(r) => r,
                Err(_) => return Ok(db_error()),
            };
            if covered(&rows, user.as_deref()) {
                if let Some(id) = upgrade_target(&rows, user.as_deref()) {
                    if db::exec(
                        &db,
                        "UPDATE shop_interest SET user_id=?1 WHERE id=?2 AND user_id IS NULL",
                        &[&db::opt_text(user.as_deref()), &db::text(id)],
                    )
                    .await
                    .is_err()
                    {
                        return Ok(db_error());
                    }
                }
                return Ok(json(&ShopInterestResponse {
                    item_id: item.id.to_string(),
                    registered: true,
                    already: true,
                }));
            }

            if db::exec(
                &db,
                "INSERT INTO shop_interest (id,item_id,catalog_version,user_id,anon_id,created_at) \
                 VALUES (?1,?2,?3,?4,?5,?6)",
                &[
                    &db::text(&uuid::random_uuid()),
                    &db::text(item.id),
                    &db::text(SHOP_CATALOG_VERSION),
                    &db::opt_text(user.as_deref()),
                    &db::opt_text(anon),
                    &db::text(&clock::now_iso()),
                ],
            )
            .await
            .is_err()
            {
                return Ok(db_error());
            }
            Ok(json(&ShopInterestResponse {
                item_id: item.id.to_string(),
                registered: true,
                already: false,
            }))
        })
        .get_async("/api/shop/interest/me", |req, ctx| async move {
            let url = match req.url() {
                Ok(u) => u,
                Err(_) => return Ok(error::error_code("Invalid URL", "INVALID_REQUEST", 400)),
            };
            let raw_anon = url
                .query_pairs()
                .find(|(k, _)| k == "anonId")
                .map(|(_, v)| v.into_owned());
            let Ok(anon) = check_anon(raw_anon.as_deref()) else {
                return Ok(invalid_anon());
            };
            let user = auth_user_opt(&req, &ctx).await;
            if user.is_none() && anon.is_none() {
                return Ok(no_identity());
            }
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(db_error()),
            };
            let rows: Vec<ItemRow> = match db::all(
                &db,
                "SELECT DISTINCT item_id FROM shop_interest WHERE user_id=?1 OR anon_id=?2",
                &[&db::opt_text(user.as_deref()), &db::opt_text(anon)],
            )
            .await
            {
                Ok(r) => r,
                Err(_) => return Ok(db_error()),
            };
            Ok(json(&ShopInterestMeResponse {
                item_ids: me_items(&rows),
            }))
        })
        .delete_async("/api/shop/interest/:itemId", |mut req, ctx| async move {
            if body_too_large(&req) {
                return Ok(payload_too_large());
            }
            let path_item = ctx.param("itemId").cloned().unwrap_or_default();
            let Some(item) = find(&path_item) else {
                return Ok(invalid_item());
            };
            // body 可省略（只靠登入）；有內容就必須是 JSON
            let text = req.text().await.unwrap_or_default();
            let body: DeleteBody = if text.trim().is_empty() {
                DeleteBody::default()
            } else {
                match serde_json::from_str(&text) {
                    Ok(b) => b,
                    Err(_) => return Ok(error::error_code("Invalid JSON", "INVALID_JSON", 400)),
                }
            };
            let Ok(anon) = check_anon(body.anon_id.as_deref()) else {
                return Ok(invalid_anon());
            };
            let user = auth_user_opt(&req, &ctx).await;
            if user.is_none() && anon.is_none() {
                return Ok(no_identity());
            }
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(db_error()),
            };
            if db::exec(
                &db,
                "DELETE FROM shop_interest WHERE item_id=?1 AND (user_id=?2 OR anon_id=?3)",
                &[
                    &db::text(item.id),
                    &db::opt_text(user.as_deref()),
                    &db::opt_text(anon),
                ],
            )
            .await
            .is_err()
            {
                return Ok(db_error());
            }
            Ok(json(&ShopInterestResponse {
                item_id: item.id.to_string(),
                registered: false,
                already: false,
            }))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anon_id_format() {
        assert!(valid_anon_id("123e4567-e89b-42d3-a456-426614174000"));
        assert!(valid_anon_id("abcdEFGH"), "下界 8");
        assert!(valid_anon_id(&"a".repeat(64)), "上界 64");
        assert!(!valid_anon_id("abcdEFG"), "7 字太短");
        assert!(!valid_anon_id(&"a".repeat(65)), "65 字太長");
        assert!(!valid_anon_id(""));
        for bad in [
            "abcd_efgh",
            "abcd efgh",
            "abcd.efgh",
            "abcdefg'",
            "抽牌抽牌抽牌抽牌",
        ] {
            assert!(!valid_anon_id(bad), "{bad}");
        }
    }

    #[test]
    fn anon_optional_but_validated() {
        assert_eq!(check_anon(None), Ok(None));
        assert_eq!(check_anon(Some("abcdefgh")), Ok(Some("abcdefgh")));
        assert_eq!(check_anon(Some("")), Err(()));
        assert_eq!(check_anon(Some("short")), Err(()));
    }

    fn row(id: &str, user: Option<&str>) -> MatchRow {
        MatchRow {
            id: id.into(),
            user_id: user.map(Into::into),
        }
    }

    #[test]
    fn upgrade_only_anon_row_when_logged_in() {
        let anon_only = [row("r1", None)];
        assert_eq!(upgrade_target(&anon_only, Some("u1")), Some("r1"));
        assert_eq!(upgrade_target(&anon_only, None), None, "匿名不補");
        let mine = [row("r1", None), row("r2", Some("u1"))];
        assert_eq!(upgrade_target(&mine, Some("u1")), None, "已有本人列不動");
        let other = [row("r1", Some("u2"))];
        assert_eq!(upgrade_target(&other, Some("u1")), None, "他人列不改");
        assert!(
            !covered(&other, Some("u1")),
            "共用瀏覽器：只命中他人列 → 本人要新增"
        );
        assert!(covered(&other, None), "匿名命中任何列即算已登記");
        assert!(covered(&anon_only, Some("u1")));
        assert!(covered(&mine, Some("u1")));
        assert!(!covered(&[], Some("u1")));
    }

    #[test]
    fn me_items_dedup_catalog_order_and_filter() {
        let rows = [
            ItemRow {
                item_id: Some("warm-lamp".into()),
            },
            ItemRow {
                item_id: Some("tarot-postcards".into()),
            },
            ItemRow {
                item_id: Some("warm-lamp".into()),
            },
            ItemRow {
                item_id: Some("retired-item".into()),
            },
            ItemRow { item_id: None },
        ];
        assert_eq!(me_items(&rows), vec!["tarot-postcards", "warm-lamp"]);
        assert!(me_items(&[]).is_empty());
    }
}
