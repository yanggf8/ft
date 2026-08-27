//! D1 bind helpers — make the worker D1 `bind_refs` ergonomic and centralize the
//! prepare → bind → await chain so routes don't repeat it. Backed by `worker::D1Type`.

use worker::{D1Database, D1Type, Result};

/// Null SQL parameter.
pub fn null() -> D1Type<'static> {
    D1Type::Null
}

/// Text SQL parameter.
pub fn text(s: &str) -> D1Type<'_> {
    D1Type::Text(s)
}

/// Integer SQL param.
pub fn int(i: i32) -> D1Type<'static> {
    D1Type::Integer(i)
}

/// Real (float) SQL param.
pub fn real(f: f64) -> D1Type<'static> {
    D1Type::Real(f)
}

/// Boolean SQL param.
pub fn bool(b: bool) -> D1Type<'static> {
    D1Type::Boolean(b)
}

/// `Option<String>` → text or NULL.
pub fn opt_text(s: Option<&str>) -> D1Type<'_> {
    match s {
        Some(v) => D1Type::Text(v),
        None => D1Type::Null,
    }
}

/// `Option<i64>` → integer or NULL (cast to i32; D1 stores as a 32-bit/float).
pub fn opt_int(v: Option<i64>) -> D1Type<'static> {
    match v {
        Some(v) => D1Type::Integer(v as i32),
        None => D1Type::Null,
    }
}

/// `Option<f64>` → real or NULL.
pub fn opt_real(v: Option<f64>) -> D1Type<'static> {
    match v {
        Some(v) => D1Type::Real(v),
        None => D1Type::Null,
    }
}

/// Run a prepared SELECT that returns the first row as `T`. `bind` is a slice of
/// `&D1Type` (0..N params). `None` when no rows.
pub async fn first<T>(db: &D1Database, sql: &str, bind: &[&D1Type<'_>]) -> Result<Option<T>>
where
    T: for<'a> serde::Deserialize<'a>,
{
    let stmt = db.prepare(sql).bind_refs(bind.iter().copied())?;
    stmt.first::<T>(None).await
}

/// Run a prepared SELECT that returns all rows as `T`.
pub async fn all<T>(db: &D1Database, sql: &str, bind: &[&D1Type<'_>]) -> Result<Vec<T>>
where
    T: for<'a> serde::Deserialize<'a>,
{
    let stmt = db.prepare(sql).bind_refs(bind.iter().copied())?;
    let result = stmt.all().await?;
    result.results::<T>()
}

/// Run a write (INSERT / UPDATE / DELETE) and return the metadata result.
pub async fn exec(db: &D1Database, sql: &str, bind: &[&D1Type<'_>]) -> Result<()> {
    let stmt = db.prepare(sql).bind_refs(bind.iter().copied())?;
    stmt.run().await.map(|_| ())
}
