//@check-pass
use dbkit::model;

#[model(table = "unicode_samples")]
pub struct UnicodeSample {
    #[key]
    pub id: i64,
    pub text: String,
    pub nullable_text: Option<String>,
    pub codepoint: i32,
    pub nullable_codepoint: Option<i32>,
}

fn nullable_text() -> dbkit::Column<UnicodeSample, Option<String>> {
    dbkit::Column::new(UnicodeSample::TABLE, "nullable_text")
}

fn nullable_codepoint() -> dbkit::Column<UnicodeSample, Option<i32>> {
    dbkit::Column::new(UnicodeSample::TABLE, "nullable_codepoint")
}

fn assert_string(_: dbkit::Expr<String>) {}
fn assert_nullable_string(_: dbkit::Expr<Option<String>>) {}
fn assert_i32(_: dbkit::Expr<i32>) {}
fn assert_nullable_i32(_: dbkit::Expr<Option<i32>>) {}
fn assert_bool(_: dbkit::Expr<bool>) {}
fn assert_nullable_bool(_: dbkit::Expr<Option<bool>>) {}

fn main() {
    use dbkit::func::NormalizationForm::{Nfc, Nfd, Nfkc, Nfkd};

    assert_string(dbkit::func::normalize(UnicodeSample::text, Nfc));
    assert_nullable_string(dbkit::func::normalize(nullable_text(), Nfd));
    assert_string(dbkit::func::normalize("①", Nfkc));
    assert_string(dbkit::func::normalize(dbkit::func::lower(UnicodeSample::text), Nfkd));
    assert_i32(dbkit::func::first_codepoint(UnicodeSample::text));
    assert_nullable_i32(dbkit::func::first_codepoint(nullable_text()));
    assert_string(dbkit::func::from_codepoint(UnicodeSample::codepoint));
    assert_nullable_string(dbkit::func::from_codepoint(nullable_codepoint()));
    assert_string(dbkit::func::to_ascii(UnicodeSample::text));
    assert_nullable_string(dbkit::func::to_ascii(nullable_text()));
    assert_string(dbkit::func::case_fold(UnicodeSample::text));
    assert_nullable_string(dbkit::func::case_fold(nullable_text()));
    assert_bool(dbkit::func::is_unicode_assigned(UnicodeSample::text));
    assert_nullable_bool(dbkit::func::is_unicode_assigned(nullable_text()));

    let normalized = dbkit::func::normalize(dbkit::func::case_fold(nullable_text()), Nfc);
    let first = dbkit::func::first_codepoint(normalized.clone());
    let _query = UnicodeSample::query()
        .select_only()
        .column_as(normalized.clone(), "normalized")
        .column_as(first.clone(), "first")
        .column_as(dbkit::func::from_codepoint(first), "round_trip")
        .filter(dbkit::func::is_unicode_assigned(normalized.clone()).eq(true))
        .order_by(dbkit::Order::asc(dbkit::func::case_fold(normalized)));
}
