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
    struct RawSale;
    let raw_sales = dbkit::Table::new("raw_sales");
    let raw_min_note_column: dbkit::Column<RawSale, Option<String>> = dbkit::Column::new(raw_sales, "note");
    let raw_max_note_column: dbkit::Column<RawSale, Option<String>> = dbkit::Column::new(raw_sales, "note");

    let first_sale_at: dbkit::Expr<Option<NaiveDateTime>> = dbkit::func::min(Sale::created_at);
    let last_sale_at: dbkit::Expr<Option<NaiveDateTime>> = dbkit::func::max(Sale::created_at);
    let min_amount: dbkit::Expr<Option<i64>> = dbkit::func::min(Sale::amount);
    let max_amount: dbkit::Expr<Option<i64>> = dbkit::func::max(Sale::amount);
    let min_note: dbkit::Expr<Option<String>> = dbkit::func::min(Sale::note);
    let max_note: dbkit::Expr<Option<String>> = dbkit::func::max(Sale::note);
    let raw_min_note: dbkit::Expr<Option<String>> = dbkit::func::min(raw_min_note_column);
    let raw_max_note: dbkit::Expr<Option<String>> = dbkit::func::max(raw_max_note_column);
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
        .column_as(raw_min_note, "raw_min_note")
        .column_as(raw_max_note, "raw_max_note")
        .column_as(us_sale_count, "us_sale_count")
        .column_as(first_us_sale_at, "first_us_sale_at")
        .group_by(Sale::region)
        .having(max_amount.gt(100_i64))
        .order_by(dbkit::Order::asc(first_sale_at));
}
