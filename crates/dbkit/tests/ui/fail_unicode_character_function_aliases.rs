fn main() {
    let _ascii = dbkit::func::ascii("A"); //~ E0425
    let _chr = dbkit::func::chr(65_i32); //~ E0425
    let _casefold = dbkit::func::casefold("Straße"); //~ E0425
    let _unicode_assigned = dbkit::func::unicode_assigned("A"); //~ E0425
}
