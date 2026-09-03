//@check-pass
use dbkit::{model, Expr, Order};

#[model(table = "sample_rows")]
pub struct SampleRow {
    #[key]
    pub id: i64,
    pub input: f64,
}

fn main() {
    let computed = Expr::<f64>::raw_sql("ABS(sample_rows.input)");

    let _query = SampleRow::query()
        .select_only()
        .column_as(computed.clone(), "computed")
        .filter(computed.clone().gt(0.01_f64))
        .order_by(Order::desc(computed));
}
