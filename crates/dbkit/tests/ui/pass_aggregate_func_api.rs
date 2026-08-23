//@check-pass
use chrono::NaiveDateTime;
use dbkit::model;

#[model(table = "sales")]
pub struct Sale {
    #[key]
    pub id: i64,
    pub region: String,
    pub note: Option<String>,
    pub small_amount: i16,
    pub nullable_small_amount: Option<i16>,
    pub integer_amount: i32,
    pub nullable_integer_amount: Option<i32>,
    pub amount: i64,
    pub nullable_amount: Option<i64>,
    pub real_amount: f32,
    pub nullable_real_amount: Option<f32>,
    pub double_amount: f64,
    pub nullable_double_amount: Option<f64>,
    pub created_at: NaiveDateTime,
}

fn main() {
    fn assert_into_expr<T>(_expr: impl dbkit::IntoExpr<T>) {}
    fn assert_aggregate<T>(_: dbkit::AggregateExpr<T>) {}

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
    assert_aggregate::<Option<i16>>(dbkit::func::sum(Sale::small_amount));
    assert_aggregate::<Option<i16>>(dbkit::func::sum(Sale::nullable_small_amount));
    assert_aggregate::<Option<i32>>(dbkit::func::sum(Sale::integer_amount));
    assert_aggregate::<Option<i32>>(dbkit::func::sum(Sale::nullable_integer_amount));
    assert_aggregate::<Option<i64>>(dbkit::func::sum(Sale::amount));
    assert_aggregate::<Option<i64>>(dbkit::func::sum(Sale::nullable_amount));
    assert_aggregate::<Option<f32>>(dbkit::func::sum(Sale::real_amount));
    assert_aggregate::<Option<f32>>(dbkit::func::sum(Sale::nullable_real_amount));
    assert_aggregate::<Option<f64>>(dbkit::func::sum(Sale::double_amount));
    assert_aggregate::<Option<f64>>(dbkit::func::sum(Sale::nullable_double_amount));
    let us_sale = Sale::region.eq("us");
    let us_sale_count: dbkit::Expr<i64> = dbkit::func::count(Sale::id).filter(us_sale.clone());
    let first_us_sale_at: dbkit::Expr<Option<NaiveDateTime>> = dbkit::func::min(Sale::created_at).filter(us_sale);
    let filtered_us_total: dbkit::Expr<Option<i64>> = dbkit::func::sum(Sale::amount).filter(Sale::region.eq("us"));
    let guaranteed_total: dbkit::Expr<i64> = dbkit::func::coalesce(dbkit::func::sum(Sale::amount), 0_i64);

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
        .column_as(filtered_us_total, "filtered_us_total")
        .column_as(guaranteed_total, "guaranteed_total")
        .group_by(Sale::region)
        .having(max_amount.gt(100_i64))
        .order_by(dbkit::Order::asc(first_sale_at));
}
