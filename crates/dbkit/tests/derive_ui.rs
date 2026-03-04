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
    config.comment_defaults.base().set_custom("rustfix", RustfixMode::Disabled);
    config.output_conflict_handling = ui_test::ignore_output_conflict;
    config.filter_files = vec![
        "basic.rs".into(),
        "pass_datetime_utc.rs".into(),
        "pass_locking_api.rs".into(),
        "fail_unloaded.rs".into(),
        "fail_conflict_cross_model.rs".into(),
        "fail_conflict_column_ref.rs".into(),
        "fail_skip_locked_without_for_update.rs".into(),
        "fail_nowait_without_for_update.rs".into(),
        "fail_for_update_after_distinct.rs".into(),
        "fail_distinct_after_for_update.rs".into(),
        "fail_for_update_skip_locked_after_distinct.rs".into(),
        "fail_for_update_nowait_after_distinct.rs".into(),
        "fail_distinct_after_for_update_skip_locked.rs".into(),
        "fail_distinct_after_for_update_nowait.rs".into(),
        "fail_for_update_after_group_by.rs".into(),
        "fail_group_by_after_for_update.rs".into(),
        "fail_for_update_after_group_by_having.rs".into(),
        "fail_group_by_having_after_for_update.rs".into(),
        "pass_conflict_update_tuple4.rs".into(),
        "pass_pgvector.rs".into(),
        "fail_pgvector_dimension_mismatch.rs".into(),
        "fail_pgvector_raw_vec.rs".into(),
        "pass_db_enum_crud.rs".into(),
        "pass_db_enum_shared_type_across_models.rs".into(),
        "fail_db_enum_value_mismatch.rs".into(),
        "fail_db_enum_optional_value_mismatch.rs".into(),
        "fail_db_enum_column_mismatch.rs".into(),
    ];
    ui_test::run_tests(config)
}
