use dbkit_core::ActiveValue;

#[test]
fn set_accepts_required_values() {
    let mut value = ActiveValue::Unset;

    value.set("required".to_string());

    assert_eq!(value, ActiveValue::Set("required".to_string()));
}

#[test]
fn set_accepts_direct_optional_and_null_values_for_nullable_fields() {
    let mut value: ActiveValue<Option<String>> = ActiveValue::Unset;

    value.set("direct".to_string());
    assert_eq!(value, ActiveValue::Set(Some("direct".to_string())));

    value.set(Some("optional".to_string()));
    assert_eq!(value, ActiveValue::Set(Some("optional".to_string())));

    value.set(None);
    assert_eq!(value, ActiveValue::Set(None));
}
