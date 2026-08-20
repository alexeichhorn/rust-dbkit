fn main() {
    let _substr = dbkit::func::substr("abcdef", 2_i32, 3_i32); //~ E0425
    let _lpad = dbkit::func::lpad("ab", 5_i32, "x"); //~ E0425
    let _rpad = dbkit::func::rpad("ab", 5_i32, "x"); //~ E0425
    let _concat_ws = dbkit::func::concat_ws(",", ["a", "b"]); //~ E0425
    let _string_to_array = dbkit::func::string_to_array("a,b", ","); //~ E0425
}
