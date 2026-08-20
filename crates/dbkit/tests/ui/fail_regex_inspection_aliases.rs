fn main() {
    let _ = dbkit::func::regexp_is_match("abc", "a"); //~ E0425
    let _ = dbkit::func::regexp_count("abc", "a"); //~ E0425
    let _ = dbkit::func::regexp_position("abc", "a"); //~ E0425
    let _ = dbkit::func::regexp_captures("abc", "a"); //~ E0425
    let _ = dbkit::func::regexp_extract("abc", "a"); //~ E0425

    let _ = dbkit::func::regexp_like("abc", "a"); //~ E0425
    let _ = dbkit::func::regexp_instr("abc", "a"); //~ E0425
    let _ = dbkit::func::regexp_match("abc", "a"); //~ E0425
    let _ = dbkit::func::regexp_substr("abc", "a"); //~ E0425
}
