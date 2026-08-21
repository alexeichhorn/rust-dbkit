//@check-pass
use dbkit::model;

#[model(table = "regex_samples")]
pub struct RegexSample {
    #[key]
    pub id: i64,
    pub source: String,
    pub pattern: String,
    pub nullable_source: Option<String>,
    pub nullable_pattern: Option<String>,
    pub number: i32,
}

fn assert_bool(_: dbkit::Expr<bool>) {}

fn assert_nullable_bool(_: dbkit::Expr<Option<bool>>) {}

fn assert_i32(_: dbkit::Expr<i32>) {}

fn assert_nullable_i32(_: dbkit::Expr<Option<i32>>) {}

fn assert_nullable_string(_: dbkit::Expr<Option<String>>) {}

fn assert_nullable_captures(_: dbkit::Expr<Option<Vec<Option<String>>>>) {}

fn main() {
    assert_bool(dbkit::func::regex_is_match(RegexSample::source, RegexSample::pattern));
    assert_nullable_bool(dbkit::func::regex_is_match(RegexSample::nullable_source, RegexSample::pattern));
    assert_nullable_bool(dbkit::func::regex_is_match(RegexSample::source, RegexSample::nullable_pattern));
    assert_nullable_bool(dbkit::func::regex_is_match(
        RegexSample::nullable_source,
        RegexSample::nullable_pattern,
    ));

    assert_i32(dbkit::func::regex_count(RegexSample::source, "a+"));
    assert_nullable_i32(dbkit::func::regex_count(
        RegexSample::nullable_source,
        dbkit::func::lower(RegexSample::pattern),
    ));
    assert_i32(dbkit::func::regex_position(
        dbkit::func::lower(RegexSample::source),
        RegexSample::pattern,
    ));
    assert_nullable_i32(dbkit::func::regex_position(RegexSample::source, RegexSample::nullable_pattern));

    assert_nullable_captures(dbkit::func::regex_captures(RegexSample::source, "(a)(b)?"));
    assert_nullable_captures(dbkit::func::regex_captures(
        RegexSample::nullable_source,
        RegexSample::nullable_pattern,
    ));
    assert_nullable_string(dbkit::func::regex_extract(RegexSample::source, RegexSample::pattern));
    assert_nullable_string(dbkit::func::regex_extract(
        RegexSample::nullable_source,
        RegexSample::nullable_pattern,
    ));

    let extracted_pattern = dbkit::func::regex_extract(dbkit::func::lower(RegexSample::source), r"[a-z]+");
    let nested_match = dbkit::func::regex_is_match(RegexSample::nullable_source, extracted_pattern.clone());
    let nested_position = dbkit::func::regex_position(RegexSample::nullable_source, extracted_pattern.clone());
    let nested_captures = dbkit::func::regex_captures(RegexSample::nullable_source, extracted_pattern);

    let _query = RegexSample::query()
        .select_only()
        .column_as(nested_match.clone(), "nested_match")
        .column_as(nested_position.clone(), "nested_position")
        .column_as(nested_captures, "nested_captures")
        .column_as(dbkit::func::regex_extract(RegexSample::source, r"^.+$"), "extract")
        .filter(nested_match.eq(true))
        .filter(dbkit::func::regex_count(RegexSample::source, r"[0-9]").gt(0_i32))
        .order_by(dbkit::Order::asc(nested_position))
        .order_by(dbkit::Order::desc(dbkit::func::regex_extract(RegexSample::nullable_source, r".+")));
}
