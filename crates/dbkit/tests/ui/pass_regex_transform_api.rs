#![allow(dead_code)]
//@check-pass
use dbkit::func::{RegexReplaceFlags, RegexSplitFlags};
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

fn assert_string_array(_: dbkit::Expr<Vec<String>>) {}

fn assert_nullable_string_array(_: dbkit::Expr<Option<Vec<String>>>) {}

// TODO(#32): Replace this with `TextSample::body` once generated columns preserve nullability.
fn nullable_body() -> dbkit::Column<TextSample, Option<String>> {
    dbkit::Column::new(TextSample::TABLE, "body")
}

fn main() {
    let all_case_insensitive = RegexReplaceFlags::GLOBAL | RegexReplaceFlags::CASE_INSENSITIVE;
    assert_string(dbkit::func::regex_replace(TextSample::title, "a", "x", RegexReplaceFlags::empty()));
    assert_nullable_string(dbkit::func::regex_replace(nullable_body(), "a", "x", all_case_insensitive));
    assert_nullable_string(dbkit::func::regex_replace(
        TextSample::title,
        nullable_body(),
        "x",
        RegexReplaceFlags::empty(),
    ));
    assert_nullable_string(dbkit::func::regex_replace(
        TextSample::title,
        "a",
        nullable_body(),
        RegexReplaceFlags::CASE_INSENSITIVE,
    ));

    assert_string_array(dbkit::func::regex_split(TextSample::title, r"\s+", RegexSplitFlags::empty()));
    assert_nullable_string_array(dbkit::func::regex_split(nullable_body(), r"\s+", RegexSplitFlags::CASE_INSENSITIVE));
    assert_nullable_string_array(dbkit::func::regex_split(
        TextSample::title,
        nullable_body(),
        RegexSplitFlags::empty(),
    ));

    let normalized = dbkit::func::regex_replace(
        dbkit::func::lower(dbkit::func::trim(dbkit::func::substring(nullable_body(), 1_i32, 99_i32))),
        dbkit::func::lower(r"[A-Z]+"),
        dbkit::func::lower("WORD"),
        all_case_insensitive,
    );
    let parts = dbkit::func::regex_split(normalized.clone(), dbkit::func::lower(r"\s+"), RegexSplitFlags::empty());
    assert_nullable_string(normalized.clone());
    assert_nullable_string_array(parts.clone());

    let _query = TextSample::query()
        .select_only()
        .column_as(normalized.clone(), "normalized")
        .column_as(parts.clone(), "parts")
        .filter(normalized.eq("word"))
        .order_by(dbkit::Order::asc(parts));
}
