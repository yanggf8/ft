//! Turso (libSQL) access over the Hrana HTTP v2 pipeline, plus the bind helpers
//! the routes are written against. The helper surface — `text`/`int`/`opt_*`
//! feeding `first`/`all`/`exec` — is unchanged from when this was backed by D1,
//! so moving off the `DB` binding did not touch the route code.
//!
//! `libsql`'s own `cloudflare` feature would have replaced this file, but it is
//! unusable here: 0.9.30 pins `worker ^0.6.7` while this crate is on 0.8, and
//! two incompatible `worker` crates cannot share one wasm binding layer. Hrana
//! over HTTP is small enough to own, and going through `worker::Fetch` keeps the
//! dependency set as it was.
//!
//! Two protocol details shape the code below:
//!
//! 1. Values are tagged (`{"type":"integer","value":"1"}`) and integers travel
//!    as *strings*, so they survive JSON's 53-bit float limit. [`decode_value`]
//!    untags them back to plain JSON, letting the existing `Deserialize` row
//!    structs work unchanged.
//! 2. `last_insert_rowid` is likewise a string, and `affected_row_count` is what
//!    D1 called `meta().changes`.

use serde_json::{json, Map, Number, Value};
use worker::wasm_bindgen::JsValue;
use worker::{Env, Error, Fetch, Headers, Method, Request, RequestInit, Result};

/// `[vars]` entry holding the database URL: `libsql://…` as `turso db show`
/// prints it, or `http://…` for a local `turso dev`.
pub const URL_VAR: &str = "TURSO_URL";
/// Secret holding the group auth token. One group token covers every database
/// in the group. Absent for a local `turso dev`, which wants no auth.
pub const AUTH_TOKEN_VAR: &str = "TURSO_AUTH_TOKEN";

fn err(message: impl Into<String>) -> Error {
    Error::RustError(message.into())
}

/// A bound SQL parameter. Mirrors the `D1Type` this replaced, including the
/// borrow, so `let t = db::text(&s); … &[&t]` at the call sites still compiles.
///
/// The full set of SQLite storage classes is kept even where no route binds one
/// yet — the constructors below are the module's API, not its call graph. As an
/// external type, `D1Type` was never dead-code analysed; this one is.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum Param<'a> {
    Null,
    Text(&'a str),
    Integer(i32),
    Real(f64),
    Boolean(bool),
}

/// Null SQL parameter.
pub fn null() -> Param<'static> {
    Param::Null
}

/// Text SQL parameter.
pub fn text(s: &str) -> Param<'_> {
    Param::Text(s)
}

/// Integer SQL param.
pub fn int(i: i32) -> Param<'static> {
    Param::Integer(i)
}

/// Real (float) SQL param.
pub fn real(f: f64) -> Param<'static> {
    Param::Real(f)
}

/// Boolean SQL param.
pub fn bool(b: bool) -> Param<'static> {
    Param::Boolean(b)
}

/// `Option<String>` → text or NULL.
pub fn opt_text(s: Option<&str>) -> Param<'_> {
    match s {
        Some(v) => Param::Text(v),
        None => Param::Null,
    }
}

/// `Option<i64>` → integer or NULL (cast to i32, as the D1-era helper did).
pub fn opt_int(v: Option<i64>) -> Param<'static> {
    match v {
        Some(v) => Param::Integer(v as i32),
        None => Param::Null,
    }
}

/// `Option<f64>` → real or NULL.
pub fn opt_real(v: Option<f64>) -> Param<'static> {
    match v {
        Some(v) => Param::Real(v),
        None => Param::Null,
    }
}

/// Handle to one libSQL database. Cheap to clone: every call is a fresh,
/// stateless pipeline request, so there is no connection to pool.
#[derive(Clone)]
pub struct Turso {
    endpoint: String,
    auth: Option<String>,
}

impl Turso {
    /// Reads `TURSO_URL` (a var) and `TURSO_AUTH_TOKEN` (a secret) off the
    /// worker env. Replaces what used to be `env.d1("DB")`.
    pub fn from_env(env: &Env) -> Result<Self> {
        let url = env
            .var(URL_VAR)
            .map_err(|_| err(format!("{URL_VAR} is not set")))?
            .to_string();
        let auth = env
            .secret(AUTH_TOKEN_VAR)
            .ok()
            .map(|secret| secret.to_string())
            .filter(|token| !token.is_empty());
        Self::new(&url, auth)
    }

    pub fn new(url: &str, auth: Option<String>) -> Result<Self> {
        Ok(Self {
            endpoint: format!("{}/v2/pipeline", http_base(url)?),
            auth,
        })
    }

    /// POST one Hrana v2 pipeline body and return the decoded payload.
    async fn post_payload(&self, body: &str) -> Result<Value> {
        let headers = Headers::new();
        headers.set("Content-Type", "application/json")?;
        if let Some(token) = &self.auth {
            headers.set("Authorization", &format!("Bearer {token}"))?;
        }

        let mut init = RequestInit::new();
        init.with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(JsValue::from_str(body)));
        let request = Request::new_with_init(&self.endpoint, &init)?;

        let mut response = Fetch::Request(request).send().await?;
        let status = response.status_code();
        if status != 200 {
            let detail = response.text().await.unwrap_or_default();
            return Err(err(format!("turso HTTP {status}: {detail}")));
        }

        response
            .json::<Value>()
            .await
            .map_err(|e| err(format!("turso response parse: {e}")))
    }

    /// POST one `execute` + `close` pipeline and return the statement result.
    async fn execute(&self, sql: &str, binds: &[&Param<'_>]) -> Result<StmtResult> {
        let args: Vec<Value> = binds.iter().map(|param| encode_value(param)).collect();
        let body = json!({
            "requests": [
                { "type": "execute", "stmt": { "sql": sql, "args": args } },
                { "type": "close" },
            ]
        })
        .to_string();
        let payload = self.post_payload(&body).await?;
        let step = payload
            .get("results")
            .and_then(Value::as_array)
            .and_then(|results| results.first())
            .ok_or_else(|| err("turso returned no result"))?;
        // A failed statement comes back as a `{"type":"error"}` step with HTTP
        // 200, so this is the only place a bad query surfaces.
        if step.get("type").and_then(Value::as_str) != Some("ok") {
            let message = step
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            return Err(err(format!("turso: {message}")));
        }
        let result = step
            .get("response")
            .and_then(|response| response.get("result"))
            .ok_or_else(|| err("turso result missing"))?;
        StmtResult::decode(result)
    }
}

/// Hrana's `StmtResult`, decoded into plain JSON rows plus the write counters.
struct StmtResult {
    rows: Vec<Value>,
    changes: usize,
}

impl StmtResult {
    /// Zips the `cols` names onto each row's positional, tagged cells.
    fn decode(result: &Value) -> Result<Self> {
        let names: Vec<String> = result
            .get("cols")
            .and_then(Value::as_array)
            .ok_or_else(|| err("turso result has no cols"))?
            .iter()
            .map(|col| {
                col.get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect();

        let mut rows = Vec::new();
        for row in result
            .get("rows")
            .and_then(Value::as_array)
            .ok_or_else(|| err("turso result has no rows"))?
        {
            let cells = row
                .as_array()
                .ok_or_else(|| err("turso row is not a list"))?;
            let mut object = Map::new();
            for (name, cell) in names.iter().zip(cells) {
                object.insert(name.clone(), decode_value(cell));
            }
            rows.push(Value::Object(object));
        }

        Ok(Self {
            rows,
            changes: result
                .get("affected_row_count")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
        })
    }
}

/// Run a prepared SELECT that returns the first row as `T`. `bind` is a slice of
/// `&Param` (0..N params). `None` when no rows.
pub async fn first<T>(db: &Turso, sql: &str, bind: &[&Param<'_>]) -> Result<Option<T>>
where
    T: for<'a> serde::Deserialize<'a>,
{
    let result = db.execute(sql, bind).await?;
    match result.rows.into_iter().next() {
        Some(row) => serde_json::from_value(row)
            .map(Some)
            .map_err(|e| err(format!("row did not deserialize: {e}"))),
        None => Ok(None),
    }
}

/// Run a prepared SELECT that returns all rows as `T`.
pub async fn all<T>(db: &Turso, sql: &str, bind: &[&Param<'_>]) -> Result<Vec<T>>
where
    T: for<'a> serde::Deserialize<'a>,
{
    let result = db.execute(sql, bind).await?;
    serde_json::from_value(Value::Array(result.rows))
        .map_err(|e| err(format!("rows did not deserialize: {e}")))
}

/// Run a write (INSERT / UPDATE / DELETE), discarding the result.
pub async fn exec(db: &Turso, sql: &str, bind: &[&Param<'_>]) -> Result<()> {
    db.execute(sql, bind).await.map(|_| ())
}

/// Run a write and report the affected row count — what D1 exposed as
/// `meta().changes`. Used by the single-use token and invite consume paths,
/// where 0 rows affected is the "already used" signal rather than an error.
pub async fn exec_changes(db: &Turso, sql: &str, bind: &[&Param<'_>]) -> Result<usize> {
    db.execute(sql, bind).await.map(|result| result.changes)
}

/// Acquire a step result's `affected_row_count`, tolerating both the plain
/// StmtResult shape and a `response/result/...`-wrapped one.
fn affected_count(item: &Value) -> usize {
    let pull = |v: &Value| {
        v.as_u64()
            .or_else(|| v.as_str().and_then(|text| text.parse().ok()))
            .unwrap_or(0) as usize
    };
    if let Some(n) = item.get("affected_row_count") {
        return pull(n);
    }
    item.pointer("/response/result/affected_row_count")
        .map(pull)
        .unwrap_or(0)
}

/// Run several statements **atomically** in one Hrana v2 `batch` — a single
/// HTTP round-trip executed in an implicit transaction; a failed step aborts
/// and rolls back the whole batch (Grok P2: F7 data-rights delete must not
/// leave ghost rows on mid-failure). Returns the affected-row count per step.
///
/// The batch result is `{"type":"success","results":[...]}` or
/// `{"type":"error", ...}`; per-step entries parse through [`affected_count`].
pub async fn batch(db: &Turso, stmts: &[(&str, &[&Param<'_>])]) -> Result<Vec<usize>, Error> {
    if stmts.is_empty() {
        return Ok(Vec::new());
    }
    let steps: Vec<Value> = stmts
        .iter()
        .map(|(sql, binds)| {
            let args: Vec<Value> = binds.iter().map(|param| encode_value(param)).collect();
            json!({ "type": "execute", "stmt": { "sql": sql, "args": args } })
        })
        .collect();
    let body = json!({
        "requests": [
            { "type": "batch", "steps": steps },
            { "type": "close" },
        ]
    })
    .to_string();
    let payload = db.post_payload(&body).await?;
    let step = payload
        .get("results")
        .and_then(Value::as_array)
        .and_then(|results| results.first())
        .ok_or_else(|| err("turso returned no result"))?;
    if step.get("type").and_then(Value::as_str) != Some("ok") {
        let message = step
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        return Err(err(format!("turso: {message}")));
    }
    let result = step
        .get("response")
        .and_then(|response| response.get("result"))
        .ok_or_else(|| err("batch result missing"))?;
    let results = match result.get("type").and_then(Value::as_str) {
        Some("success") => result
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(|| err("batch success missing results"))?,
        Some("error") => {
            let message = result
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("batch failed and was rolled back");
            return Err(err(format!("turso: {message}")));
        }
        _ => return Err(err("unexpected batch result type")),
    };
    Ok(results.iter().map(affected_count).collect())
}

/// Bind value → Hrana's tagged form. Integers go as strings, which is what the
/// protocol requires.
fn encode_value(param: &Param<'_>) -> Value {
    match param {
        Param::Null => json!({ "type": "null" }),
        Param::Text(value) => json!({ "type": "text", "value": value }),
        Param::Integer(value) => json!({ "type": "integer", "value": value.to_string() }),
        // SQLite has no boolean type; D1 bound JS `true` as 1, so match that.
        Param::Boolean(value) => {
            json!({ "type": "integer", "value": i64::from(*value).to_string() })
        }
        Param::Real(value) => json!({ "type": "float", "value": value }),
    }
}

/// Hrana's tagged form → plain JSON, so the row structs deserialize as they did
/// off D1.
fn decode_value(value: &Value) -> Value {
    match value.get("type").and_then(Value::as_str) {
        Some("integer") => value
            .get("value")
            .and_then(as_i64)
            .map_or(Value::Null, |integer| Value::Number(Number::from(integer))),
        Some("float") => value
            .get("value")
            .and_then(Value::as_f64)
            .and_then(Number::from_f64)
            .map_or(Value::Null, Value::Number),
        Some("text") => value
            .get("value")
            .and_then(Value::as_str)
            .map_or(Value::Null, |text| Value::String(text.to_owned())),
        // Blobs arrive base64 under their own key; no column here reads one.
        Some("blob") => value
            .get("base64")
            .and_then(Value::as_str)
            .map_or(Value::Null, |text| Value::String(text.to_owned())),
        _ => Value::Null,
    }
}

/// Hrana writes 64-bit integers as strings to survive JSON; accept both.
fn as_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

/// `libsql://` is the SDK scheme for what is an HTTPS endpoint; a local
/// `turso dev` is plain `http://`.
fn http_base(url: &str) -> Result<String> {
    let trimmed = url.trim().trim_end_matches('/');
    if let Some(host) = trimmed.strip_prefix("libsql://") {
        return Ok(format!("https://{host}"));
    }
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        return Ok(trimmed.to_owned());
    }
    Err(err(format!("unrecognised {URL_VAR}: {url}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affected_count_tolerates_plain_and_wrapped_shapes() {
        use serde_json::json;
        // 純 StmtResult
        assert_eq!(affected_count(&json!({"affected_row_count": 3})), 3);
        // 字串數字（Hrana 整數傳輸慣例）
        assert_eq!(affected_count(&json!({"affected_row_count": "2"})), 2);
        // wrapped: {"type":"execute","result":{"affected_row_count":1}}
        assert_eq!(
            affected_count(&json!({"response": {"result": {"affected_row_count": 1}}})),
            1
        );
        // 缺欄位 → 0
        assert_eq!(affected_count(&json!({})), 0);
    }

    #[test]
    fn http_base_maps_libsql_scheme_to_https() {
        assert_eq!(
            http_base("libsql://fortunet-yanggf8.aws-ap-northeast-1.turso.io").unwrap(),
            "https://fortunet-yanggf8.aws-ap-northeast-1.turso.io"
        );
    }

    #[test]
    fn http_base_keeps_local_turso_dev_on_http() {
        assert_eq!(
            http_base("http://127.0.0.1:8080/").unwrap(),
            "http://127.0.0.1:8080"
        );
    }

    #[test]
    fn http_base_rejects_a_bare_host() {
        assert!(http_base("fortunet.turso.io").is_err());
    }

    #[test]
    fn integers_encode_as_strings() {
        assert_eq!(
            encode_value(&Param::Integer(42)),
            json!({ "type": "integer", "value": "42" })
        );
    }

    #[test]
    fn booleans_encode_as_sqlite_integers() {
        assert_eq!(
            encode_value(&Param::Boolean(true)),
            json!({ "type": "integer", "value": "1" })
        );
    }

    #[test]
    fn null_params_encode_as_null() {
        assert_eq!(encode_value(&Param::Null), json!({ "type": "null" }));
    }

    #[test]
    fn string_integers_decode_back_to_numbers() {
        let decoded = decode_value(&json!({ "type": "integer", "value": "9007199254740993" }));
        assert_eq!(
            decoded,
            Value::Number(Number::from(9_007_199_254_740_993i64))
        );
    }

    #[test]
    fn null_cells_decode_to_json_null() {
        assert_eq!(decode_value(&json!({ "type": "null" })), Value::Null);
    }

    #[test]
    fn rows_decode_with_column_names_zipped_on() {
        let result = StmtResult::decode(&json!({
            "cols": [{ "name": "id" }, { "name": "birth_year" }],
            "rows": [[
                { "type": "text", "value": "u1" },
                { "type": "integer", "value": "1990" },
            ]],
            "affected_row_count": 0,
        }))
        .unwrap();
        assert_eq!(result.rows, vec![json!({ "id": "u1", "birth_year": 1990 })]);
    }

    #[test]
    fn affected_row_count_becomes_changes() {
        let result = StmtResult::decode(&json!({
            "cols": [],
            "rows": [],
            "affected_row_count": 3,
        }))
        .unwrap();
        assert_eq!(result.changes, 3);
    }
}
