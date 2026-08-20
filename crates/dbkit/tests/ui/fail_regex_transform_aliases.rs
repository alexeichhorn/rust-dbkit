fn main() {
    let _regexp_replace = dbkit::func::regexp_replace("abc", "b", "x", ""); //~ E0425
    let _regexp_split = dbkit::func::regexp_split("a,b", ",", ""); //~ E0425
    let _regexp_split_to_array = dbkit::func::regexp_split_to_array("a,b", ",", ""); //~ E0425
    let _regex_split_to_array = dbkit::func::regex_split_to_array("a,b", ",", "");
    //~^ E0425
}
