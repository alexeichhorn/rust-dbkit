use dbkit::model;

#[model(table = "sales")]
pub struct Sale {
    #[key]
    pub id: i64,
    pub amount: i64,
}

fn main() {
    let cast = dbkit::func::sum(Sale::amount).cast::<f64>();
    let _invalid = cast.filter(Sale::amount.gt(0_i64)); //~ E0599
}
