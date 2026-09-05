use dbkit::{func, model};

#[model(table = "method_samples")]
pub struct MethodSample {
    #[key]
    pub id: i64,
    pub text: String,
    pub count: i64,
}

fn main() {
    // FILTER must attach to the aggregate before COALESCE or a string function wraps it.
    let total = func::sum(MethodSample::count).unwrap_or(0_i64);
    let _ = total.filter(MethodSample::count.gt(0_i64)); //~ E0599
    let default_total = func::sum(MethodSample::count).unwrap_or_default();
    let _ = default_total.filter(MethodSample::count.gt(0_i64)); //~ E0599
    let trimmed = func::min(MethodSample::text).trim();
    let _ = trimmed.filter(MethodSample::count.gt(0_i64)); //~ E0599
    let lowered = func::min(MethodSample::text).lower();
    let _ = lowered.filter(MethodSample::count.gt(0_i64)); //~ E0599
    let matches = func::min(MethodSample::text).starts_with("a");
    let _ = matches.filter(MethodSample::count.gt(0_i64)); //~ E0599
}
