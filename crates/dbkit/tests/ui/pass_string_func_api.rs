//@check-pass
use dbkit::func::IntoConcatExpr;
use dbkit::model;
use dbkit::IntoExpr;

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

fn assert_string_array(_: dbkit::Expr<Vec<String>>) {}

fn assert_nullable_string_array(_: dbkit::Expr<Option<Vec<String>>>) {}

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
    assert_string(dbkit::func::title_case(TextSample::title));
    assert_nullable_string(dbkit::func::title_case(nullable_body()));
    assert_string(dbkit::func::reverse(TextSample::title));
    assert_nullable_string(dbkit::func::reverse(nullable_body()));
    assert_string(dbkit::func::replace(TextSample::title, "from", "to"));
    assert_nullable_string(dbkit::func::replace(nullable_body(), "from", "to"));
    assert_nullable_string(dbkit::func::replace(TextSample::title, nullable_body(), "to"));
    assert_nullable_string(dbkit::func::replace(TextSample::title, "from", nullable_body()));
    assert_string(dbkit::func::replace_range(TextSample::title, "replacement", 1_i32, 2_i32));
    assert_nullable_string(dbkit::func::replace_range(nullable_body(), "replacement", 1_i32, 2_i32));
    assert_nullable_string(dbkit::func::replace_range(TextSample::title, nullable_body(), 1_i32, 2_i32));
    assert_string(dbkit::func::translate_chars(TextSample::title, "from", "to"));
    assert_nullable_string(dbkit::func::translate_chars(nullable_body(), "from", "to"));
    assert_nullable_string(dbkit::func::translate_chars(TextSample::title, nullable_body(), "to"));
    assert_nullable_string(dbkit::func::translate_chars(TextSample::title, "from", nullable_body()));
    assert_string(dbkit::func::concat([TextSample::title.into_expr(), dbkit::func::lower("SUFFIX")]));
    assert_string(dbkit::func::concat([
        nullable_body().into_expr(),
        dbkit::func::trim(nullable_body()),
    ]));
    assert_string(dbkit::func::concat([
        TextSample::title.into_concat_expr(),
        nullable_body().into_concat_expr(),
    ]));
    assert_string(dbkit::func::concat([
        nullable_body().into_concat_expr(),
        "SUFFIX".into_concat_expr(),
    ]));
    assert_string(dbkit::func::concat_with_separator(
        "::",
        [TextSample::title.into_expr(), dbkit::func::lower("SUFFIX")],
    ));
    assert_string(dbkit::func::concat_with_separator(
        "::",
        [TextSample::title.into_concat_expr(), nullable_body().into_concat_expr()],
    ));
    assert_nullable_string(dbkit::func::concat_with_separator(
        nullable_body(),
        [nullable_body().into_expr(), dbkit::func::trim(nullable_body())],
    ));
    let no_values: [dbkit::Expr<String>; 0] = [];
    let no_separated_values: [dbkit::Expr<String>; 0] = [];
    assert_string(dbkit::func::concat(no_values));
    assert_string(dbkit::func::concat_with_separator("", no_separated_values));
    assert_string_array(dbkit::func::split(TextSample::title, "::"));
    assert_nullable_string_array(dbkit::func::split(nullable_body(), "::"));
    assert_string_array(dbkit::func::split(TextSample::title, nullable_body()));
    assert_string(dbkit::func::split_part(TextSample::title, "::", 1_i32));
    assert_nullable_string(dbkit::func::split_part(nullable_body(), "::", 1_i32));
    assert_nullable_string(dbkit::func::split_part(TextSample::title, nullable_body(), 1_i32));

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
            dbkit::func::char_length(TextSample::title),
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
    let nested_position = dbkit::func::position(normalized_body.clone(), dbkit::func::lower("needle"));
    let nested_prefix = dbkit::func::starts_with(normalized_body.clone(), dbkit::func::lower(TextSample::title));
    let transformed_title = dbkit::func::reverse(dbkit::func::translate_chars(
        dbkit::func::replace(
            dbkit::func::title_case(TextSample::title),
            dbkit::func::lower("FROM"),
            dbkit::func::upper("to"),
        ),
        dbkit::func::lower("ABC"),
        dbkit::func::upper("xyz"),
    ));
    let transformed_body = dbkit::func::replace_range(
        dbkit::func::title_case(nullable_body()),
        dbkit::func::reverse(TextSample::title),
        dbkit::func::char_length("x"),
        dbkit::func::char_length(TextSample::title),
    );
    assert_string(transformed_title.clone());
    assert_nullable_string(transformed_body.clone());
    let nested_part = dbkit::func::split_part(
        dbkit::func::concat([normalized_title.clone(), dbkit::func::lower("SUFFIX")]),
        "::",
        dbkit::func::char_length(TextSample::title),
    );
    let nested_parts = dbkit::func::split(normalized_body.clone(), dbkit::func::lower("::"));

    let _query = TextSample::query()
        .select_only()
        .column_as(normalized_title.clone(), "normalized_title")
        .column_as(normalized_body.clone(), "normalized_body")
        .column_as(custom_trimmed, "custom_trimmed")
        .column_as(normalized_body_len.clone(), "normalized_body_len")
        .column_as(nested_sizing, "nested_sizing")
        .column_as(repeated_suffix, "repeated_suffix")
        .column_as(dbkit::func::byte_length(normalized_body.clone()), "normalized_body_bytes")
        .column_as(dbkit::func::bit_length(normalized_body.clone()), "normalized_body_bits")
        .column_as(nested_position.clone(), "nested_position")
        .column_as(nested_prefix, "nested_prefix")
        .column_as(transformed_title.clone(), "transformed_title")
        .column_as(transformed_body.clone(), "transformed_body")
        .column_as(nested_part.clone(), "nested_part")
        .column_as(nested_parts, "nested_parts")
        .filter(TextSample::body.is_not_null())
        .filter(normalized_title.eq("alice"))
        .filter(dbkit::func::starts_with(TextSample::title, "prefix").eq(true))
        .filter(dbkit::func::position(TextSample::title, "needle").gt(0_i32))
        .filter(dbkit::func::trim_end_chars(nullable_body(), "!?").ne(""))
        .filter(dbkit::func::replace(TextSample::title, "old", "new").eq("expected"))
        .filter(dbkit::func::split_part(TextSample::title, "::", 1_i32).eq("prefix"))
        .order_by(dbkit::Order::asc(normalized_body))
        .order_by(dbkit::Order::asc(normalized_body_len))
        .order_by(dbkit::Order::asc(nested_position))
        .order_by(dbkit::Order::asc(transformed_title))
        .order_by(dbkit::Order::asc(nested_part));
}
