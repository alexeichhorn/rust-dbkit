#[test]
fn derive_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/basic.rs");
    t.pass("tests/ui/pass_datetime_utc.rs");
    t.compile_fail("tests/ui/fail_unloaded.rs");
    t.compile_fail("tests/ui/fail_conflict_cross_model.rs");
    t.compile_fail("tests/ui/fail_conflict_column_ref.rs");
}
