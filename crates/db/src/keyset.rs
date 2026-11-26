//! Keyset (cursor) paging (ADR 0030). A list is ordered by a fixed tuple of keys that ends in a
//! unique ID; a page ends at its last row's key values, and the next page reads the rows after
//! that tuple. Unlike `OFFSET`, rows added or removed before the position don't shift the next
//! page (nothing is shown twice or skipped), and the read starts at the position through an
//! index instead of stepping over every row before it.
//!
//! A list describes its order once as [`Key`]s. [`columns`] adds the key values to the select
//! list (as `keyset_0`, `keyset_1`, ...), [`order`] is the `ORDER BY`, [`after`] the condition for
//! "after this position", and [`bind`] binds the position's values to it. Keys must never be
//! NULL: a nullable column is wrapped in `coalesce` by the list.

use chrono::{DateTime, Utc};
use sqlx::postgres::{PgArguments, PgRow};
use sqlx::query::Query;
use sqlx::{FromRow, Postgres, Row};
use uuid::Uuid;

/// What a key holds, and so how its value is bound and read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyType {
    /// Any integer column, read as `bigint`.
    Int,
    /// A `real` (similarity scores).
    Real,
    Text,
    Time,
    Uuid,
    Bool,
}

impl KeyType {
    fn cast(self) -> &'static str {
        match self {
            KeyType::Int => "bigint",
            KeyType::Real => "real",
            KeyType::Text => "text",
            KeyType::Time => "timestamptz",
            KeyType::Uuid => "uuid",
            KeyType::Bool => "boolean",
        }
    }
}

/// One sort key: an SQL expression (fixed text, never user input), its type and direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    pub expr: String,
    pub ty: KeyType,
    pub desc: bool,
}

impl Key {
    pub fn asc(expr: impl Into<String>, ty: KeyType) -> Self {
        Self {
            expr: expr.into(),
            ty,
            desc: false,
        }
    }

    pub fn desc(expr: impl Into<String>, ty: KeyType) -> Self {
        Self {
            expr: expr.into(),
            ty,
            desc: true,
        }
    }
}

/// One key's value at a position.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Real(f32),
    Text(String),
    Time(DateTime<Utc>),
    Uuid(Uuid),
    Bool(bool),
}

// Scores are never NaN, so equality is total here.
impl Eq for Value {}

impl Value {
    pub fn ty(&self) -> KeyType {
        match self {
            Value::Int(_) => KeyType::Int,
            Value::Real(_) => KeyType::Real,
            Value::Text(_) => KeyType::Text,
            Value::Time(_) => KeyType::Time,
            Value::Uuid(_) => KeyType::Uuid,
            Value::Bool(_) => KeyType::Bool,
        }
    }
}

/// Whether `values` is a position in a list ordered by `keys`: as many values, of the same types.
pub fn fits(keys: &[Key], values: &[Value]) -> bool {
    keys.len() == values.len() && keys.iter().zip(values).all(|(k, v)| k.ty == v.ty())
}

/// `, <key> AS keyset_0, ...`, to append to a select list.
pub fn columns(keys: &[Key]) -> String {
    keys.iter()
        .enumerate()
        .map(|(i, k)| format!(", ({})::{} AS keyset_{i}", k.expr, k.ty.cast()))
        .collect()
}

/// The `ORDER BY` list for `keys`.
pub fn order(keys: &[Key]) -> String {
    keys.iter()
        .map(|k| format!("{} {}", k.expr, if k.desc { "DESC" } else { "ASC" }))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The condition "the row comes after the position" for `keys`, with the position's values at
/// `$first`, `$first + 1`, ... (bound by [`bind`]). When every key goes the same way it is one
/// row comparison, which an index on the keys reads as a range; otherwise the first key bounds
/// the range and the rest decide ties.
pub fn after(keys: &[Key], first: usize) -> String {
    let param = |i: usize| format!("${}::{}", first + i, keys[i].ty.cast());
    let same_way = keys.iter().all(|k| k.desc == keys[0].desc);
    if same_way {
        let cols: Vec<&str> = keys.iter().map(|k| k.expr.as_str()).collect();
        let vals: Vec<String> = (0..keys.len()).map(param).collect();
        let op = if keys[0].desc { "<" } else { ">" };
        return format!("(({}) {op} ({}))", cols.join(", "), vals.join(", "));
    }
    let past = |i: usize| if keys[i].desc { "<" } else { ">" };
    let mut alternatives = Vec::with_capacity(keys.len());
    for i in 0..keys.len() {
        let mut terms: Vec<String> = (0..i)
            .map(|j| format!("{} = {}", keys[j].expr, param(j)))
            .collect();
        terms.push(format!("{} {} {}", keys[i].expr, past(i), param(i)));
        alternatives.push(format!("({})", terms.join(" AND ")));
    }
    let bound = if keys[0].desc { "<=" } else { ">=" };
    format!(
        "({} {bound} {} AND ({}))",
        keys[0].expr,
        param(0),
        alternatives.join(" OR ")
    )
}

/// Binds a position's values, in key order, for [`after`].
pub fn bind<'q>(
    mut query: Query<'q, Postgres, PgArguments>,
    values: &'q [Value],
) -> Query<'q, Postgres, PgArguments> {
    for v in values {
        query = match v {
            Value::Int(n) => query.bind(*n),
            Value::Real(r) => query.bind(*r),
            Value::Text(t) => query.bind(t.as_str()),
            Value::Time(t) => query.bind(*t),
            Value::Uuid(u) => query.bind(*u),
            Value::Bool(b) => query.bind(*b),
        };
    }
    query
}

/// A row's key values, from the `keyset_N` columns [`columns`] added.
pub fn read(row: &PgRow, keys: &[Key]) -> Result<Vec<Value>, sqlx::Error> {
    keys.iter()
        .enumerate()
        .map(|(i, k)| {
            let col = format!("keyset_{i}");
            let col = col.as_str();
            Ok(match k.ty {
                KeyType::Int => Value::Int(row.try_get(col)?),
                KeyType::Real => Value::Real(row.try_get(col)?),
                KeyType::Text => Value::Text(row.try_get(col)?),
                KeyType::Time => Value::Time(row.try_get(col)?),
                KeyType::Uuid => Value::Uuid(row.try_get(col)?),
                KeyType::Bool => Value::Bool(row.try_get(col)?),
            })
        })
        .collect()
}

/// A page of rows and where the next one starts (`None` on the last page).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page<T> {
    pub rows: Vec<T>,
    pub next: Option<Vec<Value>>,
}

/// Decodes rows read with [`columns`] into `T` and their key values.
pub fn decode<T>(rows: &[PgRow], keys: &[Key]) -> Result<Vec<(T, Vec<Value>)>, sqlx::Error>
where
    T: for<'r> FromRow<'r, PgRow>,
{
    rows.iter()
        .map(|r| Ok((T::from_row(r)?, read(r, keys)?)))
        .collect()
}

/// The page from rows read with one more than `limit` (an after-read): the extra row only says
/// that there is a next page, which starts after the last row kept.
pub fn split<T>(mut rows: Vec<(T, Vec<Value>)>, limit: i64) -> Page<T> {
    let limit = usize::try_from(limit).unwrap_or(0);
    let more = rows.len() > limit;
    rows.truncate(limit);
    finish(rows, more)
}

/// The page from rows where the caller knows whether more follow (a numbered page: the rows
/// before it plus these are fewer than the total).
pub fn finish<T>(rows: Vec<(T, Vec<Value>)>, more: bool) -> Page<T> {
    let next = if more {
        rows.last().map(|(_, k)| k.clone())
    } else {
        None
    };
    Page {
        rows: rows.into_iter().map(|(r, _)| r).collect(),
        next,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_way_orders_are_one_row_comparison() {
        let keys = [
            Key::desc("e.created_at", KeyType::Time),
            Key::desc("e.id", KeyType::Uuid),
        ];
        assert_eq!(
            after(&keys, 3),
            "((e.created_at, e.id) < ($3::timestamptz, $4::uuid))"
        );
        assert_eq!(order(&keys), "e.created_at DESC, e.id DESC");
        assert_eq!(
            columns(&keys),
            ", (e.created_at)::timestamptz AS keyset_0, (e.id)::uuid AS keyset_1"
        );
    }

    #[test]
    fn mixed_orders_bound_the_first_key_and_break_ties() {
        let keys = [
            Key::desc("lower(e.name)", KeyType::Text),
            Key::asc("e.id", KeyType::Uuid),
        ];
        assert_eq!(
            after(&keys, 1),
            "(lower(e.name) <= $1::text AND ((lower(e.name) < $1::text) \
             OR (lower(e.name) = $1::text AND e.id > $2::uuid)))"
        );
    }

    #[test]
    fn positions_must_fit_the_keys() {
        let keys = [Key::asc("a", KeyType::Int), Key::asc("b", KeyType::Uuid)];
        assert!(fits(&keys, &[Value::Int(1), Value::Uuid(Uuid::nil())]));
        assert!(!fits(&keys, &[Value::Int(1)]));
        assert!(!fits(
            &keys,
            &[Value::Text("1".into()), Value::Uuid(Uuid::nil())]
        ));
    }

    #[test]
    fn the_extra_row_only_says_there_is_more() {
        let rows = vec![
            (1, vec![Value::Int(1)]),
            (2, vec![Value::Int(2)]),
            (3, vec![Value::Int(3)]),
        ];
        let page = split(rows.clone(), 2);
        assert_eq!(page.rows, vec![1, 2]);
        assert_eq!(page.next, Some(vec![Value::Int(2)]));
        let page = split(rows, 3);
        assert_eq!(page.rows, vec![1, 2, 3]);
        assert_eq!(page.next, None);
    }
}
