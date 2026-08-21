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

fn main() {
    assert_string(dbkit::func::concat!([]));
    assert_string(dbkit::func::concat!([TextSample::title]));
    assert_string(dbkit::func::concat!([TextSample::title, TextSample::body]));
    assert_string(dbkit::func::concat!([TextSample::body, dbkit::func::lower("SUFFIX"), "literal",]));

    assert_string(dbkit::func::concat_with_separator!("::", []));
    assert_string(dbkit::func::concat_with_separator!("::", [TextSample::title]));
    assert_string(dbkit::func::concat_with_separator!(
        "::",
        [TextSample::title, TextSample::body, "literal"],
    ));
    assert_nullable_string(dbkit::func::concat_with_separator!(
        TextSample::body,
        [TextSample::title, "literal"],
    ));

    let dynamic_values = vec![TextSample::title.into_expr(), dbkit::func::lower("SUFFIX")];
    assert_string(dbkit::func::concat!(dynamic_values));

    let dynamic_mixed_values = vec![TextSample::title.into_concat_expr(), TextSample::body.into_concat_expr()];
    assert_string(dbkit::func::concat!(dynamic_mixed_values));

    let dynamic_empty_values: Vec<dbkit::Expr<String>> = vec![];
    assert_string(dbkit::func::concat!(dynamic_empty_values));

    let dynamic_separated_values = vec![TextSample::title.into_expr(), dbkit::func::trim("body")];
    assert_string(dbkit::func::concat_with_separator!("::", dynamic_separated_values));

    let dynamic_empty_separated_values: Vec<dbkit::Expr<String>> = vec![];
    assert_string(dbkit::func::concat_with_separator!("::", dynamic_empty_separated_values,));
}
