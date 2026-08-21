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
}

fn assert_required_text_column(_: dbkit::Column<NullabilityRow, String>) {}

fn assert_nullable_text_column(_: dbkit::Column<NullabilityRow, Option<String>>) {}

fn assert_required_count_column(_: dbkit::Column<NullabilityRow, i32>) {}

fn assert_nullable_count_column(_: dbkit::Column<NullabilityRow, Option<i32>>) {}

fn assert_string(_: dbkit::Expr<String>) {}

fn assert_nullable_string(_: dbkit::Expr<Option<String>>) {}

fn assert_bool(_: dbkit::Expr<bool>) {}

fn main() {
    assert_required_text_column(NullabilityRow::required_text);
    assert_nullable_text_column(NullabilityRow::nullable_text);
    assert_required_count_column(NullabilityRow::required_count);
    assert_nullable_count_column(NullabilityRow::nullable_count);

    assert_bool(NullabilityRow::required_text.eq("required"));
    assert_bool(NullabilityRow::required_text.eq("owned required".to_string()));
    assert_bool(NullabilityRow::nullable_text.eq("present"));
    assert_bool(NullabilityRow::nullable_text.eq("owned".to_string()));
    assert_bool(NullabilityRow::nullable_text.eq(Some("present")));
    assert_bool(NullabilityRow::nullable_text.eq(Some("owned".to_string())));
    assert_bool(NullabilityRow::nullable_text.eq(None::<String>));

    let _filters = NullabilityRow::query()
        .filter(NullabilityRow::required_text.ne("other"))
        .filter(NullabilityRow::required_text.like("req%"))
        .filter(NullabilityRow::required_text.in_(["required", "other"]))
        .filter(NullabilityRow::nullable_text.ne(None::<String>))
        .filter(NullabilityRow::nullable_count.between(1_i32, 10_i32));

    assert_bool(NullabilityRow::required_text.eq_col(NullabilityRow::nullable_text));
    assert_bool(NullabilityRow::nullable_text.eq_col(NullabilityRow::required_text));
    assert_bool(NullabilityRow::required_text.ne_col(NullabilityRow::nullable_text));
    assert_bool(NullabilityRow::nullable_text.ne_col(NullabilityRow::required_text));
    assert_bool(NullabilityRow::required_text.is_distinct_from_col(NullabilityRow::nullable_text));
    assert_bool(NullabilityRow::nullable_text.is_distinct_from_col(NullabilityRow::required_text));
    assert_bool(NullabilityRow::required_text.is_not_distinct_from_col(NullabilityRow::nullable_text));
    assert_bool(NullabilityRow::nullable_text.is_not_distinct_from_col(NullabilityRow::required_text));
    assert_bool(NullabilityRow::required_count.lt_col(NullabilityRow::nullable_count));
    assert_bool(NullabilityRow::nullable_count.le_col(NullabilityRow::required_count));
    assert_bool(NullabilityRow::required_count.gt_col(NullabilityRow::nullable_count));
    assert_bool(NullabilityRow::nullable_count.ge_col(NullabilityRow::required_count));

    assert_string(dbkit::func::lower(NullabilityRow::required_text));
    assert_nullable_string(dbkit::func::lower(NullabilityRow::nullable_text));
    assert_string(dbkit::func::coalesce(NullabilityRow::nullable_text, "fallback"));
    assert_bool(dbkit::func::lower(NullabilityRow::required_text).eq_col(NullabilityRow::nullable_text));
    assert_bool(dbkit::func::lower(NullabilityRow::nullable_text).eq_col(NullabilityRow::required_text));

    let _generated_insert = NullabilityRow::insert(NullabilityRowInsert {
        id: 1,
        required_text: "required".to_string(),
        nullable_text: None,
        required_count: 1,
        nullable_count: Some(2),
    });

    let _insert_values = dbkit::Insert::<NullabilityRow>::new(NullabilityRow::TABLE)
        .value(NullabilityRow::required_text, "required")
        .value(NullabilityRow::nullable_text, "present")
        .value(NullabilityRow::nullable_count, Some(2_i32));

    let _insert_nulls = dbkit::Insert::<NullabilityRow>::new(NullabilityRow::TABLE)
        .value(NullabilityRow::nullable_text, None::<String>)
        .value(NullabilityRow::nullable_count, None::<i32>);

    let _insert_rows = dbkit::Insert::<NullabilityRow>::new(NullabilityRow::TABLE).row(|row| {
        row.value(NullabilityRow::required_text, "required")
            .value(NullabilityRow::nullable_text, None::<String>)
    });

    let _borrowed_update = NullabilityRow::update()
        .set(NullabilityRow::required_text, "updated")
        .set(NullabilityRow::nullable_text, "present");
    let _optional_borrowed_update = NullabilityRow::update().set(NullabilityRow::nullable_text, Some("updated"));
    let _owned_update = NullabilityRow::update()
        .set(NullabilityRow::required_text, "owned required".to_string())
        .set(NullabilityRow::nullable_text, "owned".to_string());
    let _optional_owned_update = NullabilityRow::update()
        .set(NullabilityRow::nullable_text, Some("owned".to_string()))
        .set(NullabilityRow::nullable_count, Some(2_i32));
    let _null_update = NullabilityRow::update()
        .set(NullabilityRow::nullable_text, None::<String>)
        .set(NullabilityRow::nullable_count, None::<i32>);

    let mut active = NullabilityRow::new_active();
    active.required_text = "required".into();
    active.nullable_text = None::<String>.into();
    active.nullable_text.set_null();
}
