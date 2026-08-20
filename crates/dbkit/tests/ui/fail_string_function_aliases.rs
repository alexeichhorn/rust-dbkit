fn main() {
    let _substr = dbkit::func::substr("abcdef", 2_i32, 3_i32); //~ E0425
    let _lpad = dbkit::func::lpad("ab", 5_i32, "x"); //~ E0425
    let _rpad = dbkit::func::rpad("ab", 5_i32, "x"); //~ E0425
    let _initcap = dbkit::func::initcap("hello world"); //~ E0425
    let _overlay = dbkit::func::overlay("abcdef", "X", 2_i32, 3_i32); //~ E0425
    let _translate = dbkit::func::translate("abc", "a", "x"); //~ E0425
}
