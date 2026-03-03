use agent_client_protocol::{
    ConfigOptionUpdate, SessionConfigGroupId, SessionConfigId, SessionConfigOption,
    SessionConfigOptionCategory, SessionConfigSelectGroup, SessionConfigSelectOption,
    SessionConfigSelectOptions, SessionConfigValueId,
};
use hermes::nvim::parse::config_option_event;

#[test]
fn test_config_option_event_ok() {
    let update = ConfigOptionUpdate::new(vec![]);

    let result = config_option_event(update);
    assert_eq!(result.get("options").is_some(), true);
}

#[test]
fn test_config_option_event_empty_config_options_array() {
    let update = ConfigOptionUpdate::new(vec![]);

    let result = config_option_event(update);
    let config_options = result.get("options").unwrap();
    assert_eq!(
        *config_options,
        nvim_oxi::Object::from(nvim_oxi::Array::new())
    );
}

#[test]
fn test_config_option_event_with_option_id_and_name() {
    let option = SessionConfigOption::select(
        SessionConfigId::new("option_1"),
        "Option One",
        SessionConfigValueId::new("default"),
        vec![SessionConfigSelectOption::new(
            SessionConfigValueId::new("default"),
            "Default",
        )],
    );
    let update = ConfigOptionUpdate::new(vec![option]);

    let result = config_option_event(update);
    let config_options = result.get("options").unwrap();

    let mut expected_option = nvim_oxi::Dictionary::new();
    expected_option.insert("id", "option_1");
    expected_option.insert("name", "Option One");

    let mut select_dict = nvim_oxi::Dictionary::new();
    select_dict.insert("currentValue", "default");

    let mut opt = nvim_oxi::Dictionary::new();
    opt.insert("value", "default");
    opt.insert("name", "Default");
    opt.insert("type", "ungrouped");
    select_dict.insert(
        "options",
        nvim_oxi::Array::from_iter([nvim_oxi::Object::from(opt)]),
    );

    expected_option.insert("kind", select_dict);

    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_option)]);
    assert_eq!(*config_options, nvim_oxi::Object::from(expected));
}

#[test]
fn test_config_option_event_with_description() {
    let option = SessionConfigOption::select(
        SessionConfigId::new("option_2"),
        "Option Two",
        SessionConfigValueId::new("default"),
        vec![SessionConfigSelectOption::new(
            SessionConfigValueId::new("default"),
            "Default",
        )],
    )
    .description("This is a description");
    let update = ConfigOptionUpdate::new(vec![option]);

    let result = config_option_event(update);
    let config_options = result.get("options").unwrap();

    let mut expected_option = nvim_oxi::Dictionary::new();
    expected_option.insert("id", "option_2");
    expected_option.insert("name", "Option Two");
    expected_option.insert("description", "This is a description");

    let mut select_dict = nvim_oxi::Dictionary::new();
    select_dict.insert("currentValue", "default");

    let mut opt = nvim_oxi::Dictionary::new();
    opt.insert("value", "default");
    opt.insert("name", "Default");
    opt.insert("type", "ungrouped");
    select_dict.insert(
        "options",
        nvim_oxi::Array::from_iter([nvim_oxi::Object::from(opt)]),
    );

    expected_option.insert("kind", select_dict);

    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_option)]);
    assert_eq!(*config_options, nvim_oxi::Object::from(expected));
}

#[test]
fn test_config_option_event_with_category() {
    let option = SessionConfigOption::select(
        SessionConfigId::new("option_3"),
        "Option Three",
        SessionConfigValueId::new("default"),
        vec![SessionConfigSelectOption::new(
            SessionConfigValueId::new("default"),
            "Default",
        )],
    )
    .category(SessionConfigOptionCategory::Model);
    let update = ConfigOptionUpdate::new(vec![option]);

    let result = config_option_event(update);
    let config_options = result.get("options").unwrap();

    let mut expected_option = nvim_oxi::Dictionary::new();
    expected_option.insert("id", "option_3");
    expected_option.insert("name", "Option Three");
    expected_option.insert("category", "Model");

    let mut select_dict = nvim_oxi::Dictionary::new();
    select_dict.insert("currentValue", "default");

    let mut opt = nvim_oxi::Dictionary::new();
    opt.insert("value", "default");
    opt.insert("name", "Default");
    opt.insert("type", "ungrouped");
    select_dict.insert(
        "options",
        nvim_oxi::Array::from_iter([nvim_oxi::Object::from(opt)]),
    );

    expected_option.insert("kind", select_dict);

    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_option)]);
    assert_eq!(*config_options, nvim_oxi::Object::from(expected));
}

#[test]
fn test_config_option_event_with_select_kind_ungrouped() {
    let option = SessionConfigOption::select(
        SessionConfigId::new("option_4"),
        "Option Four",
        SessionConfigValueId::new("value_1"),
        vec![
            SessionConfigSelectOption::new(SessionConfigValueId::new("value_1"), "Value 1"),
            SessionConfigSelectOption::new(SessionConfigValueId::new("value_2"), "Value 2"),
        ],
    );
    let update = ConfigOptionUpdate::new(vec![option]);

    let result = config_option_event(update);
    let config_options = result.get("options").unwrap();

    let mut expected_option = nvim_oxi::Dictionary::new();
    expected_option.insert("id", "option_4");
    expected_option.insert("name", "Option Four");

    let mut select_dict = nvim_oxi::Dictionary::new();
    select_dict.insert("currentValue", "value_1");

    let mut opt1 = nvim_oxi::Dictionary::new();
    opt1.insert("value", "value_1");
    opt1.insert("name", "Value 1");
    opt1.insert("type", "ungrouped");

    let mut opt2 = nvim_oxi::Dictionary::new();
    opt2.insert("value", "value_2");
    opt2.insert("name", "Value 2");
    opt2.insert("type", "ungrouped");

    select_dict.insert(
        "options",
        nvim_oxi::Array::from_iter([nvim_oxi::Object::from(opt1), nvim_oxi::Object::from(opt2)]),
    );

    expected_option.insert("kind", select_dict);

    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_option)]);
    assert_eq!(*config_options, nvim_oxi::Object::from(expected));
}

#[test]
fn test_config_option_event_with_select_kind_ungrouped_option_description() {
    let option = SessionConfigOption::select(
        SessionConfigId::new("option_5"),
        "Option Five",
        SessionConfigValueId::new("value_1"),
        vec![
            SessionConfigSelectOption::new(SessionConfigValueId::new("value_1"), "Value 1")
                .description("Option description"),
        ],
    );
    let update = ConfigOptionUpdate::new(vec![option]);

    let result = config_option_event(update);
    let config_options = result.get("options").unwrap();

    let mut expected_option = nvim_oxi::Dictionary::new();
    expected_option.insert("id", "option_5");
    expected_option.insert("name", "Option Five");

    let mut select_dict = nvim_oxi::Dictionary::new();
    select_dict.insert("currentValue", "value_1");

    let mut opt1 = nvim_oxi::Dictionary::new();
    opt1.insert("value", "value_1");
    opt1.insert("name", "Value 1");
    opt1.insert("type", "ungrouped");
    opt1.insert("description", "Option description");

    select_dict.insert(
        "options",
        nvim_oxi::Array::from_iter([nvim_oxi::Object::from(opt1)]),
    );

    expected_option.insert("kind", select_dict);

    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_option)]);
    assert_eq!(*config_options, nvim_oxi::Object::from(expected));
}

#[test]
fn test_config_option_event_with_select_kind_grouped() {
    let option = SessionConfigOption::select(
        SessionConfigId::new("option_6"),
        "Option Six",
        SessionConfigValueId::new("value_a"),
        SessionConfigSelectOptions::Grouped(vec![SessionConfigSelectGroup::new(
            SessionConfigGroupId::new("group_1"),
            "Group 1",
            vec![
                SessionConfigSelectOption::new(SessionConfigValueId::new("value_a"), "Value A"),
                SessionConfigSelectOption::new(SessionConfigValueId::new("value_b"), "Value B"),
            ],
        )]),
    );
    let update = ConfigOptionUpdate::new(vec![option]);

    let result = config_option_event(update);
    let config_options = result.get("options").unwrap();

    let mut expected_option = nvim_oxi::Dictionary::new();
    expected_option.insert("id", "option_6");
    expected_option.insert("name", "Option Six");

    let mut select_dict = nvim_oxi::Dictionary::new();
    select_dict.insert("currentValue", "value_a");

    let mut group_dict = nvim_oxi::Dictionary::new();
    group_dict.insert("type", "grouped");
    group_dict.insert("group", "group_1");
    group_dict.insert("name", "Group 1");

    let mut opt_a = nvim_oxi::Dictionary::new();
    opt_a.insert("value", "value_a");
    opt_a.insert("name", "Value A");

    let mut opt_b = nvim_oxi::Dictionary::new();
    opt_b.insert("value", "value_b");
    opt_b.insert("name", "Value B");

    group_dict.insert(
        "options",
        nvim_oxi::Array::from_iter([nvim_oxi::Object::from(opt_a), nvim_oxi::Object::from(opt_b)]),
    );

    select_dict.insert(
        "options",
        nvim_oxi::Array::from_iter([nvim_oxi::Object::from(group_dict)]),
    );

    expected_option.insert("kind", select_dict);

    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_option)]);
    assert_eq!(*config_options, nvim_oxi::Object::from(expected));
}

#[test]
fn test_config_option_event_without_meta() {
    let option = SessionConfigOption::select(
        SessionConfigId::new("option_7"),
        "Option Seven",
        SessionConfigValueId::new("default"),
        vec![SessionConfigSelectOption::new(
            SessionConfigValueId::new("default"),
            "Default",
        )],
    );
    let update = ConfigOptionUpdate::new(vec![option]);

    let result = config_option_event(update);
    assert_eq!(result.get("meta").is_some(), false);
}

#[test]
fn test_config_option_event_with_meta() {
    let option = SessionConfigOption::select(
        SessionConfigId::new("option_8"),
        "Option Eight",
        SessionConfigValueId::new("default"),
        vec![SessionConfigSelectOption::new(
            SessionConfigValueId::new("default"),
            "Default",
        )],
    );
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "agent"})
        .as_object()
        .unwrap()
        .clone();
    let update = ConfigOptionUpdate::new(vec![option]).meta(meta);

    let result = config_option_event(update);
    assert_eq!(result.get("meta").is_some(), true);
}
