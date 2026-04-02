//@check-pass
use dbkit::{model, row};

#[model(table = "lookup_items")]
pub struct LookupItem {
    #[key]
    pub id: i64,
    pub namespace: String,
    pub external_key: String,
    pub locale: String,
}

fn main() {
    let _query = LookupItem::query()
        .filter(row((LookupItem::namespace, LookupItem::external_key)).in_([
            ("public", "alpha"),
            ("internal", "beta"),
        ]))
        .filter(row((LookupItem::namespace, LookupItem::external_key, LookupItem::locale)).in_([
            ("public", "alpha", "en"),
            ("internal", "beta", "de"),
        ]));
}
