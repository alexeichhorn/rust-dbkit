use dbkit::model;

type OptionalText = Option<String>;

#[model(table = "aliased_nullable_rows")] //~ ERROR: trait bound
pub struct AliasedNullableRow {
    #[key]
    pub id: i64,
    pub nullable_text: OptionalText,
}

fn main() {}
