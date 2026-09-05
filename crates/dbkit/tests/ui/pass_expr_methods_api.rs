//@check-pass
use dbkit::{func, model, Expr, IntoExpr, Order};

#[model(table = "method_samples")]
pub struct MethodSample {
    #[key]
    pub id: i64,
    pub text: String,
    pub optional_text: Option<String>,
    pub prefix: String,
    pub optional_prefix: Option<String>,
    pub count: i32,
    pub optional_count: Option<i32>,
    pub optional_bool: Option<bool>,
    pub optional_bytes: Option<Vec<u8>>,
    pub optional_tags: Option<Vec<String>>,
}

fn main() {
    use MethodSample as S;

    let _: Expr<String> = S::optional_text.unwrap_or("fallback");
    let _: Expr<String> = S::optional_text.unwrap_or(String::from("owned"));
    let _: Expr<String> = S::optional_text.unwrap_or(S::text);
    let _: Expr<String> = S::optional_text.into_expr().unwrap_or(S::text.lower());
    let _: Expr<i32> = S::optional_count.unwrap_or(0);
    let _: Expr<i32> = (S::optional_count + 1_i32).unwrap_or(S::count + 2_i32);
    let _: Expr<bool> = S::optional_bool.unwrap_or(false);
    let _: Expr<bool> = S::optional_text.eq("ready").unwrap_or(false);

    let _: Expr<String> = S::optional_text.unwrap_or_default();
    let _: Expr<String> = S::optional_text.into_expr().unwrap_or_default();
    let _: Expr<i32> = S::optional_count.unwrap_or_default();
    let _: Expr<i32> = (S::optional_count + 1_i32).unwrap_or_default();
    let _: Expr<bool> = S::optional_bool.unwrap_or_default();
    let _: Expr<bool> = S::optional_text.eq("ready").unwrap_or_default();
    let _: Expr<Vec<u8>> = S::optional_bytes.unwrap_or_default();
    let _: Expr<Vec<String>> = S::optional_tags.into_expr().unwrap_or_default();

    let _: Expr<String> = S::text.trim();
    let _: Expr<String> = S::text.lower();
    let _: Expr<String> = S::text.into_expr().trim();
    let _: Expr<String> = S::text.into_expr().lower();
    let _: Expr<Option<String>> = S::optional_text.trim();
    let _: Expr<Option<String>> = S::optional_text.lower();
    let _: Expr<Option<String>> = S::optional_text.into_expr().trim();
    let _: Expr<Option<String>> = S::optional_text.into_expr().lower();
    let _: Expr<Option<String>> = S::optional_text.trim().lower();
    let _: Expr<String> = S::optional_text.unwrap_or_default().trim().lower();
    let _: Expr<String> = S::optional_text.trim().lower().unwrap_or_default();

    // Either nullable operand makes STARTS_WITH nullable, including a nullable prefix.
    let _: Expr<bool> = S::text.starts_with("a");
    let _: Expr<bool> = S::text.starts_with(String::from("a"));
    let _: Expr<bool> = S::text.starts_with(S::prefix);
    let _: Expr<Option<bool>> = S::text.starts_with(S::optional_prefix);
    let _: Expr<Option<bool>> = S::optional_text.starts_with(S::prefix);
    let _: Expr<Option<bool>> = S::optional_text.starts_with(S::optional_prefix);
    let _: Expr<bool> = S::text.lower().starts_with(S::prefix.lower());
    let _: Expr<Option<bool>> = S::text.lower().starts_with(S::optional_prefix.lower());
    let _: Expr<Option<bool>> = S::optional_text.lower().starts_with(S::prefix.lower());
    let _: Expr<Option<bool>> = S::optional_text.lower().starts_with(S::optional_prefix.lower());
    let _: Expr<bool> = S::optional_text.trim().lower().starts_with("a").unwrap_or(false);

    // Aggregate inputs work too; wrapping them produces an ordinary scalar expression.
    let _: Expr<i32> = func::sum(S::count).unwrap_or(0);
    let _: Expr<i32> = func::sum(S::count).filter(S::count.gt(0)).unwrap_or_default();
    let _: Expr<Option<String>> = func::min(S::text).trim();
    let _: Expr<Option<String>> = func::min(S::text).lower();
    let _: Expr<Option<bool>> = func::min(S::text).starts_with("a");

    let normalized = S::optional_text.trim().lower().unwrap_or_default();
    let _query = S::query()
        .filter(normalized.clone().starts_with("a"))
        .order_by(Order::asc(normalized.clone()))
        .select_only()
        .column_as(normalized, "normalized");
}
