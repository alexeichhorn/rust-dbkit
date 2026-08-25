//@check-pass
use dbkit::{model, row};

#[model(table = "lookup_items")]
pub struct LookupItem {
    #[key]
    pub id: i64,
    pub namespace: String,
    pub optional_namespace: Option<String>,
    pub external_key: String,
    pub optional_external_key: Option<String>,
    pub locale: String,
    pub optional_locale: Option<String>,
}

fn assert_bool(_: dbkit::Expr<bool>) {}

fn assert_nullable_bool(_: dbkit::Expr<Option<bool>>) {}

fn main() {
    assert_bool(row((LookupItem::namespace, LookupItem::external_key)).in_([("public", "alpha")]));
    assert_nullable_bool(row((LookupItem::optional_namespace, LookupItem::external_key)).in_([("public", "alpha")]));
    assert_nullable_bool(row((LookupItem::namespace, LookupItem::optional_external_key)).in_([("public", "alpha")]));
    assert_nullable_bool(row((LookupItem::optional_namespace, LookupItem::optional_external_key)).in_([
        (Some("public".to_string()), Some("alpha".to_string())),
        (Some("internal".to_string()), None),
    ]));
    assert_nullable_bool(
        row((LookupItem::namespace, LookupItem::optional_external_key, LookupItem::locale)).in_([("public", "alpha", "en")]),
    );
    assert_nullable_bool(
        row((LookupItem::namespace, LookupItem::external_key, LookupItem::optional_locale)).in_(std::iter::empty::<(&str, &str, &str)>()),
    );

    let _query = LookupItem::query()
        .filter(row((LookupItem::namespace, LookupItem::external_key)).in_([("public", "alpha"), ("internal", "beta")]))
        .filter(
            row((LookupItem::namespace, LookupItem::external_key, LookupItem::locale))
                .in_([("public", "alpha", "en"), ("internal", "beta", "de")]),
        )
        .filter(
            row((LookupItem::namespace, LookupItem::optional_external_key))
                .in_([("public", Some("alpha".to_string())), ("internal", None)]),
        );
}
