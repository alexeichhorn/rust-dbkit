use dbkit::Expr;

fn raw_expression(sql: &str) {
    let _expression = Expr::<f64>::raw_sql(sql); //~ E0521
}

fn main() {}
