//@check-pass
use dbkit::model;

#[model(table = "text_samples")]
pub struct TextSample {
    #[key]
    pub id: i64,
    pub title: String,
    pub body: Option<String>,
}

fn assert_string(_: dbkit::Expr<String>) {}

fn assert_nullable_string(_: dbkit::Expr<Option<String>>) {}

fn main() {
    assert_string(dbkit::func::lower(TextSample::title));
    assert_nullable_string(dbkit::func::lower(TextSample::body));
    assert_string(dbkit::func::trim_chars(TextSample::title, "xy"));
    assert_nullable_string(dbkit::func::trim_chars(TextSample::body, "xy"));
    assert_string(dbkit::func::trim_start(TextSample::title));
    assert_nullable_string(dbkit::func::trim_start(TextSample::body));
    assert_string(dbkit::func::trim_start_chars(TextSample::title, "xy"));
    assert_nullable_string(dbkit::func::trim_start_chars(TextSample::body, "xy"));
    assert_string(dbkit::func::trim_end(TextSample::title));
    assert_nullable_string(dbkit::func::trim_end(TextSample::body));
    assert_string(dbkit::func::trim_end_chars(TextSample::title, "xy"));
    assert_nullable_string(dbkit::func::trim_end_chars(TextSample::body, "xy"));

    let normalized_title = dbkit::func::lower(dbkit::func::trim(dbkit::func::trim_start_chars(
        dbkit::func::trim(TextSample::title),
        "@",
    )));
    let normalized_body = dbkit::func::lower(dbkit::func::trim_end(dbkit::func::trim_start(TextSample::body)));
    let nested_character_expression = dbkit::func::lower("XY");
    let custom_trimmed = dbkit::func::trim_chars(TextSample::title, nested_character_expression);
    let normalized_body_len = dbkit::func::char_length(normalized_body.clone());

    let _query = TextSample::query()
        .select_only()
        .column_as(normalized_title.clone(), "normalized_title")
        .column_as(normalized_body.clone(), "normalized_body")
        .column_as(custom_trimmed, "custom_trimmed")
        .column_as(normalized_body_len.clone(), "normalized_body_len")
        .filter(TextSample::body.is_not_null())
        .filter(normalized_title.eq("alice"))
        .filter(dbkit::func::trim_end_chars(TextSample::body, "!?").ne(""))
        .order_by(dbkit::Order::asc(normalized_body))
        .order_by(dbkit::Order::asc(normalized_body_len));
}
