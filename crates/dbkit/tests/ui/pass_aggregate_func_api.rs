//@check-pass
use chrono::NaiveDateTime;
use dbkit::model;

#[model(table = "sales")]
pub struct Sale {
    #[key]
    pub id: i64,
    pub region: String,
    pub amount: i64,
    pub created_at: NaiveDateTime,
}

fn main() {
    let first_sale_at: dbkit::Expr<Option<NaiveDateTime>> = dbkit::func::min(Sale::created_at);
    let last_sale_at: dbkit::Expr<Option<NaiveDateTime>> = dbkit::func::max(Sale::created_at);
    let min_amount: dbkit::Expr<Option<i64>> = dbkit::func::min(Sale::amount);
    let max_amount: dbkit::Expr<Option<i64>> = dbkit::func::max(Sale::amount);

    let _query = Sale::query()
        .select_only()
        .column_as(Sale::region, "region")
        .column_as(first_sale_at.clone(), "first_sale_at")
        .column_as(last_sale_at, "last_sale_at")
        .column_as(min_amount, "min_amount")
        .column_as(max_amount.clone(), "max_amount")
        .group_by(Sale::region)
        .having(max_amount.gt(100_i64))
        .order_by(dbkit::Order::asc(first_sale_at));
}
