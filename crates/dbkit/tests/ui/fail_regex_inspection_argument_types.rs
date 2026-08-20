use dbkit::model;

#[model(table = "regex_samples")]
pub struct RegexSample {
    #[key]
    pub id: i64,
    pub source: String,
    pub pattern: String,
    pub number: i32,
}

fn main() {
    let _non_text_source = dbkit::func::regex_is_match(RegexSample::number, "[0-9]"); //~ E0277
    let _non_text_pattern = dbkit::func::regex_count(RegexSample::source, RegexSample::number); //~ E0277
    let _non_text_extract = dbkit::func::regex_extract(RegexSample::number, RegexSample::pattern);
    //~ E0277
}
