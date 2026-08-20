use dbkit::func::{RegexReplaceFlags, RegexSplitFlags};
use dbkit::model;

#[model(table = "metrics")]
pub struct Metric {
    #[key]
    pub id: i64,
    pub label: String,
    pub attempts: i32,
}

fn main() {
    let _non_text_source = dbkit::func::regex_replace(Metric::attempts, "a", "x", RegexReplaceFlags::empty()); //~ E0277
    let _non_text_pattern = dbkit::func::regex_replace(Metric::label, Metric::attempts, "x", RegexReplaceFlags::empty()); //~ E0277
    let _non_text_replacement = dbkit::func::regex_replace(Metric::label, "a", Metric::attempts, RegexReplaceFlags::empty()); //~ E0277
    let _string_replace_flags = dbkit::func::regex_replace(Metric::label, "a", "x", "g"); //~ E0308
    let _string_split_flags = dbkit::func::regex_split(Metric::label, "a", "i"); //~ E0308
    let _replace_flags_on_split = dbkit::func::regex_split(Metric::label, "a", RegexReplaceFlags::GLOBAL); //~ E0308
    let _split_flags_on_replace = dbkit::func::regex_replace(Metric::label, "a", "x", RegexSplitFlags::CASE_INSENSITIVE);
    //~^ E0308
}
