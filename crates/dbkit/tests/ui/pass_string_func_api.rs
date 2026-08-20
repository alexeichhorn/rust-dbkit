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

fn assert_i32(_: dbkit::Expr<i32>) {}

fn assert_nullable_i32(_: dbkit::Expr<Option<i32>>) {}

fn assert_bool(_: dbkit::Expr<bool>) {}

fn assert_nullable_bool(_: dbkit::Expr<Option<bool>>) {}

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
    assert_i32(dbkit::func::byte_length(TextSample::title));
    assert_nullable_i32(dbkit::func::byte_length(nullable_body()));
    assert_i32(dbkit::func::bit_length("UTF-8"));
    assert_nullable_i32(dbkit::func::bit_length(dbkit::func::trim(nullable_body())));
    assert_i32(dbkit::func::position(TextSample::title, "needle"));
    assert_nullable_i32(dbkit::func::position(nullable_body(), TextSample::title));
    assert_nullable_i32(dbkit::func::position(TextSample::title, nullable_body()));
    assert_nullable_i32(dbkit::func::position(nullable_body(), nullable_body()));
    assert_bool(dbkit::func::starts_with(TextSample::title, "prefix"));
    assert_nullable_bool(dbkit::func::starts_with(nullable_body(), TextSample::title));
    assert_nullable_bool(dbkit::func::starts_with(TextSample::title, nullable_body()));
    assert_nullable_bool(dbkit::func::starts_with(nullable_body(), nullable_body()));

    let normalized_title = dbkit::func::lower(dbkit::func::trim(dbkit::func::trim_start_chars(
        dbkit::func::trim(TextSample::title),
        "@",
    )));
    let normalized_body = dbkit::func::lower(dbkit::func::trim_end(dbkit::func::trim_start(nullable_body())));
    let nested_character_expression = dbkit::func::lower("XY");
    let custom_trimmed = dbkit::func::trim_chars(TextSample::title, nested_character_expression);
    let normalized_body_len = dbkit::func::char_length(normalized_body.clone());
    let nested_position = dbkit::func::position(normalized_body.clone(), dbkit::func::lower("needle"));
    let nested_prefix = dbkit::func::starts_with(normalized_body.clone(), dbkit::func::lower(TextSample::title));

    let _query = TextSample::query()
        .select_only()
        .column_as(normalized_title.clone(), "normalized_title")
        .column_as(normalized_body.clone(), "normalized_body")
        .column_as(custom_trimmed, "custom_trimmed")
        .column_as(normalized_body_len.clone(), "normalized_body_len")
        .column_as(dbkit::func::byte_length(normalized_body.clone()), "normalized_body_bytes")
        .column_as(dbkit::func::bit_length(normalized_body.clone()), "normalized_body_bits")
        .column_as(nested_position.clone(), "nested_position")
        .column_as(nested_prefix, "nested_prefix")
        .filter(TextSample::body.is_not_null())
        .filter(normalized_title.eq("alice"))
        .filter(dbkit::func::starts_with(TextSample::title, "prefix").eq(true))
        .filter(dbkit::func::position(TextSample::title, "needle").gt(0_i32))
        .filter(dbkit::func::trim_end_chars(nullable_body(), "!?").ne(""))
        .order_by(dbkit::Order::asc(normalized_body))
        .order_by(dbkit::Order::asc(normalized_body_len))
        .order_by(dbkit::Order::asc(nested_position));
}
