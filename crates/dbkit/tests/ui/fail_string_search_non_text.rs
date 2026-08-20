use dbkit::model;

#[model(table = "text_samples")]
pub struct TextSample {
    #[key]
    pub id: i64,
    pub title: String,
    pub attempts: i32,
}

fn main() {
    let _invalid_expression = dbkit::func::position(TextSample::attempts, "1"); //~ E0277
    let _prefix = dbkit::func::starts_with(TextSample::title, TextSample::attempts);
    //~^ E0277
}
