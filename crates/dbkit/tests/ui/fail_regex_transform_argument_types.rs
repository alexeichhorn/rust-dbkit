use dbkit::model;

#[model(table = "metrics")]
pub struct Metric {
    #[key]
    pub id: i64,
    pub label: String,
    pub attempts: i32,
}

fn main() {
    let _non_text_source = dbkit::func::regex_replace(Metric::attempts, "a", "x", ""); //~ E0277
    let _non_text_pattern = dbkit::func::regex_replace(Metric::label, Metric::attempts, "x", ""); //~ E0277
    let _non_text_replacement = dbkit::func::regex_replace(Metric::label, "a", Metric::attempts, ""); //~ E0277
    let _non_text_flags = dbkit::func::regex_split(Metric::label, "a", Metric::attempts);
    //~^ E0277
}
