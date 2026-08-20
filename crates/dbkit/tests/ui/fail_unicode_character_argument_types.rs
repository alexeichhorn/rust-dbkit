use dbkit::model;

#[model(table = "unicode_samples")]
pub struct UnicodeSample {
    #[key]
    pub id: i64,
    pub text: String,
    pub codepoint: i32,
}

fn main() {
    let _non_text = dbkit::func::normalize(UnicodeSample::codepoint, dbkit::func::NormalizationForm::Nfc); //~ E0277
    let _unchecked_form = dbkit::func::normalize(UnicodeSample::text, "NFC"); //~ E0308
    let _non_text_codepoint = dbkit::func::first_codepoint(UnicodeSample::codepoint); //~ E0277
    let _bigint_character = dbkit::func::from_codepoint(65_i64); //~ E0277
    let _non_text_assignment_check = dbkit::func::is_unicode_assigned(UnicodeSample::codepoint);
    //~^ E0277
}
