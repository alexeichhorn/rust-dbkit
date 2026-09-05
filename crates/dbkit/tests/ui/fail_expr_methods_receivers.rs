use dbkit::{model, IntoExpr, PgVector};

#[model(table = "method_samples")]
pub struct MethodSample {
    #[key]
    pub id: i64,
    pub text: String,
    pub count: i32,
    pub optional_count: Option<i32>,
    pub embedding: Option<PgVector<3>>,
}

fn main() {
    // Unwrapping is only available on nullable columns and expressions.
    let _ = MethodSample::text.unwrap_or("fallback"); //~ E0599
    let _ = MethodSample::text.into_expr().unwrap_or("fallback"); //~ E0599
    let _ = MethodSample::count.unwrap_or_default(); //~ E0599
    let _ = MethodSample::count.into_expr().unwrap_or_default(); //~ E0599

    let _ = MethodSample::count.trim(); //~ E0599
    let _ = MethodSample::count.lower(); //~ E0599
    let _ = MethodSample::count.starts_with("1"); //~ E0599
    let _ = MethodSample::optional_count.into_expr().trim(); //~ E0599
    let _ = MethodSample::optional_count.into_expr().lower(); //~ E0599
    let _ = MethodSample::optional_count.into_expr().starts_with("1"); //~ E0599

    // A SQL-convertible type still needs a Rust Default for unwrap_or_default.
    let _ = MethodSample::embedding.unwrap_or_default(); //~ ERROR: Default
    let _ = MethodSample::embedding.into_expr().unwrap_or_default(); //~ ERROR: Default
}
