fn main() {
    let _replace_without_flags = dbkit::func::regex_replace("abc", "b", "x"); //~ E0061
    let _split_without_flags = dbkit::func::regex_split("a,b", ","); //~ E0061
}
