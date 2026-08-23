//@check-pass
use dbkit::model;

#[model(table = "nullability_rows")]
pub struct NullabilityRow {
    #[key]
    pub id: i64,
    pub required_text: String,
    pub nullable_text: Option<String>,
    pub required_count: i32,
    pub nullable_count: Option<i32>,
    pub nullable_limit: Option<i32>,
}

fn assert_required_text_column(_: dbkit::Column<NullabilityRow, String>) {}

fn assert_nullable_text_column(_: dbkit::Column<NullabilityRow, Option<String>>) {}

fn assert_required_count_column(_: dbkit::Column<NullabilityRow, i32>) {}

fn assert_nullable_count_column(_: dbkit::Column<NullabilityRow, Option<i32>>) {}

fn assert_string(_: dbkit::Expr<String>) {}

fn assert_nullable_string(_: dbkit::Expr<Option<String>>) {}

fn assert_i32(_: dbkit::Expr<i32>) {}

fn assert_nullable_i32(_: dbkit::Expr<Option<i32>>) {}

fn assert_bool(_: dbkit::Expr<bool>) {}

fn assert_nullable_bool(_: dbkit::Expr<Option<bool>>) {}

fn main() {
    assert_required_text_column(NullabilityRow::required_text);
    assert_nullable_text_column(NullabilityRow::nullable_text);
    assert_required_count_column(NullabilityRow::required_count);
    assert_nullable_count_column(NullabilityRow::nullable_count);
    assert_nullable_count_column(NullabilityRow::nullable_limit);

    assert_bool(NullabilityRow::required_text.eq("required"));
    assert_bool(NullabilityRow::required_text.eq("owned required".to_string()));
    assert_nullable_bool(NullabilityRow::nullable_text.eq("present"));
    assert_nullable_bool(NullabilityRow::nullable_text.eq("owned".to_string()));
    assert_nullable_bool(NullabilityRow::nullable_text.eq(Some("owned".to_string())));
    assert_nullable_bool(NullabilityRow::nullable_text.eq(None));
    assert_nullable_bool(NullabilityRow::nullable_text.ne("present"));
    assert_nullable_bool(NullabilityRow::nullable_text.ne(Some("owned".to_string())));
    assert_nullable_bool(NullabilityRow::nullable_text.ne(None));
    assert_nullable_bool(NullabilityRow::nullable_count.lt(5_i32));
    assert_nullable_bool(NullabilityRow::nullable_count.lt(Some(5_i32)));
    assert_nullable_bool(NullabilityRow::nullable_count.lt(None));
    assert_nullable_bool(NullabilityRow::nullable_count.le(5_i32));
    assert_nullable_bool(NullabilityRow::nullable_count.le(Some(5_i32)));
    assert_nullable_bool(NullabilityRow::nullable_count.le(None));
    assert_nullable_bool(NullabilityRow::nullable_count.gt(5_i32));
    assert_nullable_bool(NullabilityRow::nullable_count.gt(Some(5_i32)));
    assert_nullable_bool(NullabilityRow::nullable_count.gt(None));
    assert_nullable_bool(NullabilityRow::nullable_count.ge(5_i32));
    assert_nullable_bool(NullabilityRow::nullable_count.ge(Some(5_i32)));
    assert_nullable_bool(NullabilityRow::nullable_count.ge(None));
    assert_nullable_bool(NullabilityRow::nullable_count.between(1_i32, 10_i32));
    assert_nullable_bool(NullabilityRow::nullable_count.between(Some(1_i32), Some(10_i32)));
    assert_nullable_bool(NullabilityRow::nullable_count.between(None, Some(10_i32)));
    assert_nullable_bool(NullabilityRow::nullable_text.like("pre%"));
    assert_nullable_bool(NullabilityRow::nullable_text.like(Some("pre%".to_string())));
    assert_nullable_bool(NullabilityRow::nullable_text.like(None));
    assert_nullable_bool(NullabilityRow::nullable_text.ilike("PRE%"));
    assert_nullable_bool(NullabilityRow::nullable_text.in_(["present", "other"]));
    assert_nullable_bool(NullabilityRow::nullable_text.in_([Some("present".to_string()), None]));
    assert_nullable_bool(NullabilityRow::nullable_text.in_(std::iter::empty::<String>()));
    assert_bool(NullabilityRow::nullable_text.is_null());
    assert_bool(NullabilityRow::nullable_text.is_not_null());

    let _filters = NullabilityRow::query()
        .filter(NullabilityRow::required_text.ne("other"))
        .filter(NullabilityRow::required_text.like("req%"))
        .filter(NullabilityRow::required_text.in_(["required", "other"]))
        .filter(NullabilityRow::nullable_text.ne(None))
        .filter(NullabilityRow::nullable_count.between(1_i32, 10_i32));

    assert_bool(NullabilityRow::required_text.eq_col(NullabilityRow::required_text));
    assert_bool(NullabilityRow::required_text.ne_col(NullabilityRow::required_text));
    assert_nullable_bool(NullabilityRow::required_text.eq_col(NullabilityRow::nullable_text));
    assert_nullable_bool(NullabilityRow::nullable_text.eq_col(NullabilityRow::required_text));
    assert_nullable_bool(NullabilityRow::nullable_text.eq_col(NullabilityRow::nullable_text));
    assert_nullable_bool(NullabilityRow::required_text.ne_col(NullabilityRow::nullable_text));
    assert_nullable_bool(NullabilityRow::nullable_text.ne_col(NullabilityRow::required_text));
    assert_nullable_bool(NullabilityRow::nullable_text.ne_col(NullabilityRow::nullable_text));
    assert_bool(NullabilityRow::required_text.is_distinct_from_col(NullabilityRow::nullable_text));
    assert_bool(NullabilityRow::nullable_text.is_distinct_from_col(NullabilityRow::required_text));
    assert_bool(NullabilityRow::nullable_text.is_distinct_from_col(NullabilityRow::nullable_text));
    assert_bool(NullabilityRow::required_text.is_not_distinct_from_col(NullabilityRow::nullable_text));
    assert_bool(NullabilityRow::nullable_text.is_not_distinct_from_col(NullabilityRow::required_text));
    assert_bool(NullabilityRow::nullable_text.is_not_distinct_from_col(NullabilityRow::nullable_text));
    assert_bool(NullabilityRow::required_count.lt_col(NullabilityRow::required_count));
    assert_bool(NullabilityRow::required_count.le_col(NullabilityRow::required_count));
    assert_bool(NullabilityRow::required_count.gt_col(NullabilityRow::required_count));
    assert_bool(NullabilityRow::required_count.ge_col(NullabilityRow::required_count));
    assert_nullable_bool(NullabilityRow::required_count.lt_col(NullabilityRow::nullable_count));
    assert_nullable_bool(NullabilityRow::nullable_count.lt_col(NullabilityRow::required_count));
    assert_nullable_bool(NullabilityRow::nullable_count.lt_col(NullabilityRow::nullable_limit));
    assert_nullable_bool(NullabilityRow::required_count.le_col(NullabilityRow::nullable_count));
    assert_nullable_bool(NullabilityRow::nullable_count.le_col(NullabilityRow::required_count));
    assert_nullable_bool(NullabilityRow::nullable_count.le_col(NullabilityRow::nullable_limit));
    assert_nullable_bool(NullabilityRow::required_count.gt_col(NullabilityRow::nullable_count));
    assert_nullable_bool(NullabilityRow::nullable_count.gt_col(NullabilityRow::required_count));
    assert_nullable_bool(NullabilityRow::nullable_count.gt_col(NullabilityRow::nullable_limit));
    assert_nullable_bool(NullabilityRow::required_count.ge_col(NullabilityRow::nullable_count));
    assert_nullable_bool(NullabilityRow::nullable_count.ge_col(NullabilityRow::required_count));
    assert_nullable_bool(NullabilityRow::nullable_count.ge_col(NullabilityRow::nullable_limit));

    assert_string(dbkit::func::lower(NullabilityRow::required_text));
    assert_nullable_string(dbkit::func::lower(NullabilityRow::nullable_text));
    assert_nullable_bool(dbkit::func::lower(NullabilityRow::nullable_text).eq("present"));
    assert_nullable_bool(dbkit::func::lower(NullabilityRow::nullable_text).eq(None));
    assert_nullable_bool(dbkit::func::lower(NullabilityRow::nullable_text).ne("present"));
    assert_nullable_bool(dbkit::func::lower(NullabilityRow::nullable_text).like("pre%"));
    assert_nullable_bool(dbkit::func::lower(NullabilityRow::nullable_text).ilike("PRE%"));
    assert_nullable_bool(dbkit::func::lower(NullabilityRow::nullable_text).in_(["present", "other"]));
    assert_string(dbkit::func::coalesce(NullabilityRow::nullable_text, "fallback"));
    assert_string(dbkit::func::coalesce(NullabilityRow::required_text, NullabilityRow::nullable_text));
    assert_nullable_string(dbkit::func::coalesce(NullabilityRow::nullable_text, NullabilityRow::nullable_text));
    assert_string(dbkit::func::coalesce(NullabilityRow::required_text, NullabilityRow::required_text));

    // PostgreSQL GREATEST and LEAST ignore NULL arguments, so one required argument guarantees a required result.
    assert_i32(dbkit::func::greatest(
        NullabilityRow::required_count,
        NullabilityRow::required_count,
    ));
    assert_i32(dbkit::func::least(NullabilityRow::required_count, NullabilityRow::required_count));
    assert_i32(dbkit::func::greatest(NullabilityRow::nullable_count, 0_i32));
    assert_i32(dbkit::func::greatest(0_i32, NullabilityRow::nullable_count));
    assert_i32(dbkit::func::least(NullabilityRow::nullable_count, 100_i32));
    assert_i32(dbkit::func::least(100_i32, NullabilityRow::nullable_count));
    assert_i32(dbkit::func::greatest(
        NullabilityRow::nullable_count,
        NullabilityRow::required_count,
    ));
    assert_i32(dbkit::func::greatest(
        NullabilityRow::required_count,
        NullabilityRow::nullable_count,
    ));
    assert_i32(dbkit::func::least(NullabilityRow::required_count, NullabilityRow::nullable_count));
    assert_i32(dbkit::func::least(NullabilityRow::nullable_count, NullabilityRow::required_count));
    assert_nullable_i32(dbkit::func::greatest(
        NullabilityRow::nullable_count,
        NullabilityRow::nullable_limit,
    ));
    assert_nullable_i32(dbkit::func::least(NullabilityRow::nullable_count, NullabilityRow::nullable_limit));
    assert_string(dbkit::func::greatest(NullabilityRow::nullable_text, "fallback"));
    assert_nullable_string(dbkit::func::least(NullabilityRow::nullable_text, NullabilityRow::nullable_text));

    assert_nullable_bool(dbkit::func::lower(NullabilityRow::required_text).eq_col(NullabilityRow::nullable_text));
    assert_nullable_bool(dbkit::func::lower(NullabilityRow::nullable_text).eq_col(NullabilityRow::required_text));

    let nullable_count_expr = NullabilityRow::nullable_count + NullabilityRow::required_count;
    assert_nullable_bool(nullable_count_expr.clone().eq_col(NullabilityRow::required_count));
    assert_nullable_bool(nullable_count_expr.clone().ne_col(NullabilityRow::required_count));
    assert_nullable_bool(nullable_count_expr.clone().lt_col(NullabilityRow::required_count));
    assert_nullable_bool(nullable_count_expr.clone().le_col(NullabilityRow::required_count));
    assert_nullable_bool(nullable_count_expr.clone().gt_col(NullabilityRow::required_count));
    assert_nullable_bool(nullable_count_expr.ge_col(NullabilityRow::required_count));

    let nullable_comparison = NullabilityRow::nullable_text.eq_col(NullabilityRow::required_text);
    let required_comparison = NullabilityRow::required_text.eq_col(NullabilityRow::required_text);
    assert_nullable_bool(nullable_comparison.clone().and(required_comparison.clone()));
    assert_nullable_bool(required_comparison.clone().and(nullable_comparison.clone()));
    assert_nullable_bool(
        nullable_comparison
            .clone()
            .and(NullabilityRow::nullable_text.ne_col(NullabilityRow::required_text)),
    );
    assert_nullable_bool(nullable_comparison.clone().or(required_comparison.clone()));
    assert_nullable_bool(required_comparison.or(nullable_comparison.clone()));
    assert_nullable_bool(
        nullable_comparison
            .clone()
            .or(NullabilityRow::nullable_text.ne_col(NullabilityRow::required_text)),
    );
    assert_nullable_bool(nullable_comparison.clone().not());

    let _nullable_predicates = NullabilityRow::query()
        .filter(nullable_comparison.clone())
        .filter(nullable_comparison.clone().and(NullabilityRow::id.gt(0_i64)));
    let _nullable_join = NullabilityRow::query().join_on(NullabilityRow::TABLE, nullable_comparison.clone());
    let _nullable_left_join = NullabilityRow::query().left_join_on(NullabilityRow::TABLE, nullable_comparison.clone());
    let _nullable_having = NullabilityRow::query()
        .group_by(NullabilityRow::required_text)
        .having(nullable_comparison.clone());
    let _nullable_condition = dbkit::Condition::all().add(nullable_comparison.clone()).into_expr();
    let _filtered_aggregate = dbkit::func::count(NullabilityRow::id).filter(nullable_comparison.clone());
    let _filtered_update = NullabilityRow::update()
        .set(NullabilityRow::required_text, "updated")
        .filter(nullable_comparison.clone());
    let _filtered_delete = NullabilityRow::delete().filter(nullable_comparison);

    let _generated_insert = NullabilityRow::insert(NullabilityRowInsert {
        id: 1,
        required_text: "required".to_string(),
        nullable_text: None,
        required_count: 1,
        nullable_count: Some(2),
        nullable_limit: None,
    });

    let _insert_values = dbkit::Insert::<NullabilityRow>::new(NullabilityRow::TABLE)
        .value(NullabilityRow::required_text, "required")
        .value(NullabilityRow::nullable_text, "present")
        .value(NullabilityRow::nullable_count, Some(2_i32));

    let _insert_nulls = dbkit::Insert::<NullabilityRow>::new(NullabilityRow::TABLE)
        .value(NullabilityRow::nullable_text, None)
        .value(NullabilityRow::nullable_count, None::<i32>);

    let _insert_rows = dbkit::Insert::<NullabilityRow>::new(NullabilityRow::TABLE).row(|row| {
        row.value(NullabilityRow::required_text, "required")
            .value(NullabilityRow::nullable_text, None)
    });

    let _borrowed_update = NullabilityRow::update()
        .set(NullabilityRow::required_text, "updated")
        .set(NullabilityRow::nullable_text, "present");
    let _owned_update = NullabilityRow::update()
        .set(NullabilityRow::required_text, "owned required".to_string())
        .set(NullabilityRow::nullable_text, "owned".to_string());
    let _optional_owned_update = NullabilityRow::update()
        .set(NullabilityRow::nullable_text, Some("owned".to_string()))
        .set(NullabilityRow::nullable_count, Some(2_i32));
    let _null_update = NullabilityRow::update()
        .set(NullabilityRow::nullable_text, None)
        .set(NullabilityRow::nullable_count, None::<i32>);

    let mut active = NullabilityRow::new_active();
    active.required_text = "required".into();
    active.nullable_text = "present".into();
    active.nullable_text = "owned".to_string().into();
    active.nullable_text = Some("optional".to_string()).into();
    active.nullable_text = None.into();
    active.nullable_text = dbkit::ActiveValue::Set(None);
    active.nullable_text = dbkit::ActiveValue::Unchanged(None);
    active.nullable_text.set_null();
    active.required_text.set("required".to_string());
    active.nullable_text.set("direct".to_string());
    active.nullable_text.set(Some("optional".to_string()));
    active.nullable_text.set(None);
    active.required_count.set(1_i32);
    active.nullable_count.set(2_i32);
    active.nullable_count.set(Some(3_i32));
    active.nullable_count.set(None);
}
