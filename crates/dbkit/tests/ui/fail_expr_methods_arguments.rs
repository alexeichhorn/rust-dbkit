use dbkit::{model, IntoExpr};

#[model(table = "method_samples")]
pub struct MethodSample {
    #[key]
    pub id: i64,
    pub text: String,
    pub optional_text: Option<String>,
    pub count: i32,
    pub optional_count: Option<i32>,
}

fn main() {
    let _ = MethodSample::optional_text.unwrap_or(1_i32); //~ E0277
    let _ = MethodSample::optional_count.into_expr().unwrap_or("zero"); //~ E0277

    // A nullable fallback cannot guarantee the required return type.
    let _ = MethodSample::optional_text.unwrap_or(MethodSample::optional_text); //~ E0277
    let _ = MethodSample::optional_text
        .into_expr()
        .unwrap_or(MethodSample::optional_text.into_expr()); //~ E0277
    let _ = MethodSample::optional_text.unwrap_or(None::<String>); //~ E0277

    let _ = MethodSample::text.starts_with(1_i32); //~ E0277
    let _ = MethodSample::optional_text.into_expr().starts_with(MethodSample::count); //~ E0277
    let _ = MethodSample::text.into_expr().starts_with(MethodSample::optional_count.into_expr());
    //~^ E0277
}
