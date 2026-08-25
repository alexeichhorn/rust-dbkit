//@check-pass
use dbkit::model;

type RequiredText = String;

#[model(table = "direct_option_rows")]
pub struct DirectOptionRow {
    #[key]
    pub id: i64,
    pub required_text: RequiredText,
    pub nullable_text: Option<String>,
    pub qualified_nullable_count: std::option::Option<i32>,
}

fn assert_required_text(_: dbkit::Column<DirectOptionRow, String>) {}

fn assert_nullable_text(_: dbkit::Column<DirectOptionRow, Option<String>>) {}

fn assert_nullable_count(_: dbkit::Column<DirectOptionRow, Option<i32>>) {}

fn main() {
    assert_required_text(DirectOptionRow::required_text);
    assert_nullable_text(DirectOptionRow::nullable_text);
    assert_nullable_count(DirectOptionRow::qualified_nullable_count);

    let _insert = DirectOptionRow::insert(DirectOptionRowInsert {
        id: 1,
        required_text: "required".to_string(),
        nullable_text: None,
        qualified_nullable_count: Some(2),
    });
}
