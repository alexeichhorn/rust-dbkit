use dbkit::model;

#[model(table = "text_samples")]
pub struct TextSample {
    #[key]
    pub id: i64,
    pub title: String,
}

fn main() {
    let _both = dbkit::func::trim_chars(TextSample::title, 1_i32); //~ E0277
    let _start = dbkit::func::trim_start_chars(TextSample::title, 1_i32); //~ E0277
    let _end = dbkit::func::trim_end_chars(TextSample::title, 1_i32); //~ E0277
}
