#![allow(non_upper_case_globals)]

use dbkit::{model, row, DbEnum, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum)]
#[dbkit(type_name = "lookup_scope", rename_all = "snake_case")]
pub enum LookupScope {
    Public,
    Internal,
}

#[model(table = "lookup_records")]
pub struct LookupRecord {
    #[key]
    pub id: i64,
    pub scope: LookupScope,
    pub external_key: String,
    pub locale: String,
    pub label: String,
}

#[test]
fn enum_row_value_in_sql_uses_typed_casts_and_bind_reuse() {
    let compiled = LookupRecord::query()
        .filter(
            row((LookupRecord::scope, LookupRecord::external_key, LookupRecord::locale))
                .in_([(LookupScope::Public, "alpha", "en"), (LookupScope::Public, "beta", "de")]),
        )
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT lookup_records.* FROM lookup_records WHERE ((lookup_records.scope, lookup_records.external_key, lookup_records.locale) IN (($1::lookup_scope, $2, $3), ($1::lookup_scope, $4, $5)))"
    );
    assert_eq!(
        compiled.binds,
        vec![
            Value::Enum {
                type_name: "lookup_scope",
                value: "public".to_string(),
            },
            Value::String("alpha".to_string()),
            Value::String("en".to_string()),
            Value::String("beta".to_string()),
            Value::String("de".to_string()),
        ]
    );
}

#[test]
fn enum_row_value_in_empty_compiles_to_false_without_binds() {
    let compiled = LookupRecord::query()
        .filter(
            row((LookupRecord::scope, LookupRecord::external_key, LookupRecord::locale))
                .in_(std::iter::empty::<(LookupScope, &str, &str)>()),
        )
        .compile();

    assert_eq!(compiled.sql, "SELECT lookup_records.* FROM lookup_records WHERE (FALSE)");
    assert!(compiled.binds.is_empty());
}
