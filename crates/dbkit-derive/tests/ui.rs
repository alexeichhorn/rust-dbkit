#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/basic.rs");
    t.compile_fail("tests/ui/fail_unloaded.rs");
}
