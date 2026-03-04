use std::path::PathBuf;
use ui_test::custom_flags::rustfix::RustfixMode;
use ui_test::dependencies::DependencyBuilder;

fn main() -> ui_test::color_eyre::Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut config = ui_test::Config::rustc("tests/ui");
    config.out_dir = manifest_dir.join("../../target/ui-dbkit");
    config.comment_defaults.base().set_custom(
        "dependencies",
        DependencyBuilder {
            crate_manifest_path: PathBuf::from("tests/ui/Cargo.toml"),
            ..DependencyBuilder::default()
        },
    );
    config
        .comment_defaults
        .base()
        .set_custom("rustfix", RustfixMode::Disabled);
    config.output_conflict_handling = ui_test::ignore_output_conflict;
    config.filter_files = vec![
        "basic.rs".into(),
        "pass_datetime_utc.rs".into(),
        "pass_pgvector.rs".into(),
        "fail_unloaded.rs".into(),
        "fail_conflict_cross_model.rs".into(),
        "fail_conflict_column_ref.rs".into(),
        "fail_pgvector_dimension_mismatch.rs".into(),
        "fail_pgvector_raw_vec.rs".into(),
    ];
    ui_test::run_tests(config)
}
