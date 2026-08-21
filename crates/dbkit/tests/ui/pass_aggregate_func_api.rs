//@check-pass
use chrono::NaiveDateTime;
use dbkit::model;

#[model(table = "sales")]
pub struct Sale {
    #[key]
    pub id: i64,
    pub region: String,
    pub note: Option<String>,
    pub amount: i64,
    pub created_at: NaiveDateTime,
}

fn main() {
    fn assert_into_expr<T>(_expr: impl dbkit::IntoExpr<T>) {}

    let first_sale_at = dbkit::func::min(Sale::created_at);
    let last_sale_at = dbkit::func::max(Sale::created_at);
    let min_amount = dbkit::func::min(Sale::amount);
    let max_amount = dbkit::func::max(Sale::amount);
    let min_note = dbkit::func::min(Sale::note);
    let max_note = dbkit::func::max(Sale::note);
    assert_into_expr::<Option<NaiveDateTime>>(first_sale_at.clone());
    assert_into_expr::<Option<NaiveDateTime>>(last_sale_at.clone());
    assert_into_expr::<Option<i64>>(min_amount.clone());
    assert_into_expr::<Option<i64>>(max_amount.clone());
    assert_into_expr::<Option<String>>(min_note.clone());
    assert_into_expr::<Option<String>>(max_note.clone());
    let us_sale = Sale::region.eq("us");
    let us_sale_count: dbkit::Expr<i64> = dbkit::func::count(Sale::id).filter(us_sale.clone());
    let first_us_sale_at: dbkit::Expr<Option<NaiveDateTime>> = dbkit::func::min(Sale::created_at).filter(us_sale);

    let _query = Sale::query()
        .select_only()
        .column_as(Sale::region, "region")
        .column_as(first_sale_at.clone(), "first_sale_at")
        .column_as(last_sale_at, "last_sale_at")
        .column_as(min_amount, "min_amount")
        .column_as(max_amount.clone(), "max_amount")
        .column_as(min_note, "min_note")
        .column_as(max_note, "max_note")
        .column_as(us_sale_count, "us_sale_count")
        .column_as(first_us_sale_at, "first_us_sale_at")
        .group_by(Sale::region)
        .having(max_amount.gt(100_i64))
        .order_by(dbkit::Order::asc(first_sale_at));
}
