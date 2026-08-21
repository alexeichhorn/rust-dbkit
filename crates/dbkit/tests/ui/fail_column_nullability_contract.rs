use dbkit::model;

#[model(table = "nullability_rows")]
pub struct NullabilityRow {
    #[key]
    pub id: i64,
    pub required_text: String,
    pub nullable_text: Option<String>,
}

fn require_non_nullable_column(_: dbkit::Column<NullabilityRow, String>) {}

fn require_nullable_column(_: dbkit::Column<NullabilityRow, Option<String>>) {}

fn require_non_nullable_expression(_: dbkit::Expr<String>) {}

fn require_nullable_expression(_: dbkit::Expr<Option<String>>) {}

fn main() {
    require_non_nullable_column(NullabilityRow::nullable_text); //~ E0308
    require_nullable_column(NullabilityRow::required_text); //~ E0308
    require_non_nullable_expression(dbkit::func::lower(NullabilityRow::nullable_text)); //~ E0308
    require_nullable_expression(dbkit::func::lower(NullabilityRow::required_text));
    //~^ E0308
    let _borrowed_optional = NullabilityRow::nullable_text.eq(Some("present")); //~ E0277
}
