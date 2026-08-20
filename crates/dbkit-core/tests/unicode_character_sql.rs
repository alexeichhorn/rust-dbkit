use dbkit_core::{func, Column, Expr, Order, Select, Table, Value};

#[derive(Debug)]
struct TextSample;

fn text_samples_table() -> Table {
    Table::new("text_samples")
}

fn title() -> Column<TextSample, String> {
    Column::new(text_samples_table(), "title")
}

fn body() -> Column<TextSample, Option<String>> {
    Column::new(text_samples_table(), "body")
}

fn codepoint() -> Column<TextSample, i32> {
    Column::new(text_samples_table(), "codepoint")
}

fn nullable_codepoint() -> Column<TextSample, Option<i32>> {
    Column::new(text_samples_table(), "nullable_codepoint")
}

#[test]
fn compiles_unicode_character_functions_with_postgresql_names_and_types() {
    let normalized_title: Expr<String> = func::normalize(title(), func::NormalizationForm::Nfc);
    let normalized_body: Expr<Option<String>> = func::normalize(body(), func::NormalizationForm::Nfd);
    let compatibility_composed: Expr<String> = func::normalize("①", func::NormalizationForm::Nfkc);
    let compatibility_decomposed: Expr<String> = func::normalize(func::lower(title()), func::NormalizationForm::Nfkd);
    let title_codepoint: Expr<i32> = func::first_codepoint(title());
    let body_codepoint: Expr<Option<i32>> = func::first_codepoint(body());
    let character: Expr<String> = func::from_codepoint(codepoint());
    let nullable_character: Expr<Option<String>> = func::from_codepoint(nullable_codepoint());
    let ascii_body: Expr<Option<String>> = func::to_ascii(body());
    let folded_title: Expr<String> = func::case_fold(title());
    let assigned_body: Expr<Option<bool>> = func::is_unicode_assigned(body());

    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column_as(normalized_title, "normalized_title")
        .column_as(normalized_body, "normalized_body")
        .column_as(compatibility_composed, "compatibility_composed")
        .column_as(compatibility_decomposed, "compatibility_decomposed")
        .column_as(title_codepoint, "title_codepoint")
        .column_as(body_codepoint, "body_codepoint")
        .column_as(character, "character")
        .column_as(nullable_character, "nullable_character")
        .column_as(ascii_body, "ascii_body")
        .column_as(folded_title, "folded_title")
        .column_as(assigned_body, "assigned_body");

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT NORMALIZE(text_samples.title, NFC) AS normalized_title, NORMALIZE(text_samples.body, NFD) AS normalized_body, NORMALIZE($1, NFKC) AS compatibility_composed, NORMALIZE(LOWER(text_samples.title), NFKD) AS compatibility_decomposed, ASCII(text_samples.title) AS title_codepoint, ASCII(text_samples.body) AS body_codepoint, CHR(text_samples.codepoint) AS character, CHR(text_samples.nullable_codepoint) AS nullable_character, TO_ASCII(text_samples.body) AS ascii_body, CASEFOLD(text_samples.title) AS folded_title, UNICODE_ASSIGNED(text_samples.body) AS assigned_body FROM text_samples"
    );
    assert_eq!(sql.binds, vec![Value::String("①".to_string())]);
}

#[test]
fn keeps_unicode_text_bound_while_composing_in_queries() {
    let unsafe_text = "'%_\\.*[]()";
    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column_as(func::normalize(unsafe_text, func::NormalizationForm::Nfc), "safe_text")
        .column_as(func::from_codepoint(39_i32), "quote")
        .filter(func::first_codepoint(func::normalize(title(), func::NormalizationForm::Nfd)).eq(39_i32))
        .filter(func::is_unicode_assigned(func::normalize(body(), func::NormalizationForm::Nfkc)).eq(true))
        .order_by(Order::asc(func::case_fold(func::normalize(body(), func::NormalizationForm::Nfkd))));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT NORMALIZE($1, NFC) AS safe_text, CHR($2) AS quote FROM text_samples WHERE (ASCII(NORMALIZE(text_samples.title, NFD)) = $2) AND (UNICODE_ASSIGNED(NORMALIZE(text_samples.body, NFKC)) = $3) ORDER BY CASEFOLD(NORMALIZE(text_samples.body, NFKD)) ASC"
    );
    assert_eq!(
        sql.binds,
        vec![Value::String(unsafe_text.to_string()), Value::I32(39), Value::Bool(true),]
    );
}
