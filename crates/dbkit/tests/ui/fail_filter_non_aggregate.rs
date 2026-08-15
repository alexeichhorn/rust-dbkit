use dbkit::model;

#[model(table = "sales")]
pub struct Sale {
    #[key]
    pub id: i64,
    pub region: String,
    pub amount: i64,
}

fn main() {
    let positive_amount = Sale::amount.gt(0_i64);

    let _scalar_function = dbkit::func::upper(Sale::region).filter(positive_amount.clone()); //~ E0599

    let _wrapped_aggregate = dbkit::func::coalesce(dbkit::func::sum(Sale::amount), 0_i64).filter(positive_amount.clone()); //~ E0599

    let filtered_count = dbkit::func::count(Sale::id).filter(Sale::region.eq("us"));
    let _second_filter = filtered_count.filter(positive_amount); //~ E0599
}
