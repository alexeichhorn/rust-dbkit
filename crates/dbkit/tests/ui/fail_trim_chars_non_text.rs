use dbkit::model;

#[model(table = "text_samples")]
pub struct TextSample {
    #[key]
    pub id: i64,
    pub title: String,
}

fn main() {
    let _expr = dbkit::func::trim_chars(TextSample::title, 1_i32); //~ E0277
}
