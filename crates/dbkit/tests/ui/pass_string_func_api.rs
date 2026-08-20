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

// TODO(#32): Replace this with `TextSample::body` once generated columns preserve nullability.
fn nullable_body() -> dbkit::Column<TextSample, Option<String>> {
    dbkit::Column::new(TextSample::TABLE, "body")
}

fn main() {
    assert_string(dbkit::func::lower(TextSample::title));
    assert_nullable_string(dbkit::func::lower(nullable_body()));
    assert_string(dbkit::func::trim_chars(TextSample::title, "xy"));
    assert_nullable_string(dbkit::func::trim_chars(nullable_body(), "xy"));
    assert_string(dbkit::func::trim_start(TextSample::title));
    assert_nullable_string(dbkit::func::trim_start(nullable_body()));
    assert_string(dbkit::func::trim_start_chars(TextSample::title, "xy"));
    assert_nullable_string(dbkit::func::trim_start_chars(nullable_body(), "xy"));
    assert_string(dbkit::func::trim_end(TextSample::title));
    assert_nullable_string(dbkit::func::trim_end(nullable_body()));
    assert_string(dbkit::func::trim_end_chars(TextSample::title, "xy"));
    assert_nullable_string(dbkit::func::trim_end_chars(nullable_body(), "xy"));
    assert_string(dbkit::func::left(TextSample::title, 2_i32));
    assert_nullable_string(dbkit::func::left(nullable_body(), 2_i32));
    assert_string(dbkit::func::right(TextSample::title, 2_i32));
    assert_nullable_string(dbkit::func::right(nullable_body(), 2_i32));
    assert_string(dbkit::func::substring(TextSample::title, 2_i32, 3_i32));
    assert_nullable_string(dbkit::func::substring(nullable_body(), 2_i32, 3_i32));
    assert_string(dbkit::func::repeat(TextSample::title, dbkit::func::char_length(TextSample::title)));
    assert_nullable_string(dbkit::func::repeat(nullable_body(), 2_i32));
    assert_string(dbkit::func::pad_start(
        TextSample::title,
        dbkit::func::char_length(TextSample::title),
        "xy",
    ));
    assert_nullable_string(dbkit::func::pad_start(nullable_body(), 8_i32, "xy"));
    assert_string(dbkit::func::pad_end(TextSample::title, 8_i32, "xy"));
    assert_nullable_string(dbkit::func::pad_end(nullable_body(), 8_i32, "xy"));

    let normalized_title = dbkit::func::lower(dbkit::func::trim(dbkit::func::trim_start_chars(
        dbkit::func::trim(TextSample::title),
        "@",
    )));
    let normalized_body = dbkit::func::lower(dbkit::func::trim_end(dbkit::func::trim_start(nullable_body())));
    let nested_character_expression = dbkit::func::lower("XY");
    let custom_trimmed = dbkit::func::trim_chars(TextSample::title, nested_character_expression);
    let normalized_body_len = dbkit::func::char_length(normalized_body.clone());
    let nested_sizing = dbkit::func::pad_end(
        dbkit::func::substring(
            dbkit::func::lower(dbkit::func::trim(TextSample::title)),
            dbkit::func::char_length("x"),
            normalized_body_len.clone(),
        ),
        dbkit::func::char_length(TextSample::title),
        dbkit::func::lower("XY"),
    );
    let repeated_suffix = dbkit::func::repeat(
        dbkit::func::right(nullable_body(), dbkit::func::char_length(TextSample::title)),
        2_i32,
    );
    assert_string(nested_sizing.clone());
    assert_nullable_string(repeated_suffix.clone());

    let _query = TextSample::query()
        .select_only()
        .column_as(normalized_title.clone(), "normalized_title")
        .column_as(normalized_body.clone(), "normalized_body")
        .column_as(custom_trimmed, "custom_trimmed")
        .column_as(normalized_body_len.clone(), "normalized_body_len")
        .column_as(nested_sizing, "nested_sizing")
        .column_as(repeated_suffix, "repeated_suffix")
        .filter(TextSample::body.is_not_null())
        .filter(normalized_title.eq("alice"))
        .filter(dbkit::func::trim_end_chars(nullable_body(), "!?").ne(""))
        .order_by(dbkit::Order::asc(normalized_body))
        .order_by(dbkit::Order::asc(normalized_body_len));
}
