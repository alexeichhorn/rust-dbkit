//@check-pass
use dbkit::model;

#[model(table = "text_samples")]
pub struct TextSample {
    #[key]
    pub id: i64,
    pub title: String,
    pub body: Option<String>,
}

fn main() {
    let trimmed_title = dbkit::func::trim(TextSample::title);
    let trimmed_body = dbkit::func::trim(TextSample::body);
    let trimmed_body_len = dbkit::func::char_length(trimmed_body.clone());

    let _query = TextSample::query()
        .select_only()
        .column_as(trimmed_title.clone(), "trimmed_title")
        .column_as(trimmed_body.clone(), "trimmed_body")
        .column_as(trimmed_body_len.clone(), "trimmed_body_len")
        .filter(TextSample::body.is_not_null())
        .filter(dbkit::func::char_length(dbkit::func::trim(TextSample::body)).ge(5_i32))
        .order_by(dbkit::Order::asc(trimmed_title))
        .order_by(dbkit::Order::asc(trimmed_body_len));
}
