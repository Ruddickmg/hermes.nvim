use agent_client_protocol::{Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus};
use hermes::nvim::parse::plan_event;

#[test]
fn test_plan_event_ok() {
    let entry = PlanEntry::new(
        "Analyze codebase",
        PlanEntryPriority::High,
        PlanEntryStatus::Pending,
    );
    let plan = Plan::new(vec![entry]);

    let result = plan_event(plan);
    assert_eq!(result.get("entries").is_some(), true);
}

#[test]
fn test_plan_event_empty_entries() {
    let plan = Plan::new(vec![]);

    let result = plan_event(plan);
    let entries = result.get("entries").unwrap();
    assert_eq!(*entries, nvim_oxi::Object::from(nvim_oxi::Array::new()));
}

#[test]
fn test_plan_event_single_entry() {
    let entry = PlanEntry::new(
        "Analyze codebase",
        PlanEntryPriority::High,
        PlanEntryStatus::Pending,
    );
    let plan = Plan::new(vec![entry]);

    let result = plan_event(plan);
    let entries = result.get("entries").unwrap();

    let mut expected_entry = nvim_oxi::Dictionary::new();
    expected_entry.insert("content", "Analyze codebase");
    expected_entry.insert("priority", "High");
    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_entry)]);
    assert_eq!(*entries, nvim_oxi::Object::from(expected));
}

#[test]
fn test_plan_event_multiple_entries() {
    let entry1 = PlanEntry::new(
        "First task",
        PlanEntryPriority::High,
        PlanEntryStatus::Pending,
    );
    let entry2 = PlanEntry::new(
        "Second task",
        PlanEntryPriority::Low,
        PlanEntryStatus::InProgress,
    );
    let plan = Plan::new(vec![entry1, entry2]);

    let result = plan_event(plan);
    let entries = result.get("entries").unwrap();

    let mut expected_entry1 = nvim_oxi::Dictionary::new();
    expected_entry1.insert("content", "First task");
    expected_entry1.insert("priority", "High");

    let mut expected_entry2 = nvim_oxi::Dictionary::new();
    expected_entry2.insert("content", "Second task");
    expected_entry2.insert("priority", "Low");

    let expected = nvim_oxi::Array::from_iter([
        nvim_oxi::Object::from(expected_entry1),
        nvim_oxi::Object::from(expected_entry2),
    ]);
    assert_eq!(*entries, nvim_oxi::Object::from(expected));
}

#[test]
fn test_plan_event_entry_content_value() {
    let entry = PlanEntry::new(
        "My task content",
        PlanEntryPriority::Medium,
        PlanEntryStatus::Pending,
    );
    let plan = Plan::new(vec![entry]);

    let result = plan_event(plan);
    let entries = result.get("entries").unwrap();

    let mut expected_entry = nvim_oxi::Dictionary::new();
    expected_entry.insert("content", "My task content");
    expected_entry.insert("priority", "Medium");
    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_entry)]);
    assert_eq!(*entries, nvim_oxi::Object::from(expected));
}

#[test]
fn test_plan_event_entry_priority_high() {
    let entry = PlanEntry::new("Task", PlanEntryPriority::High, PlanEntryStatus::Pending);
    let plan = Plan::new(vec![entry]);

    let result = plan_event(plan);
    let entries = result.get("entries").unwrap();

    let mut expected_entry = nvim_oxi::Dictionary::new();
    expected_entry.insert("content", "Task");
    expected_entry.insert("priority", "High");
    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_entry)]);
    assert_eq!(*entries, nvim_oxi::Object::from(expected));
}

#[test]
fn test_plan_event_entry_priority_medium() {
    let entry = PlanEntry::new("Task", PlanEntryPriority::Medium, PlanEntryStatus::Pending);
    let plan = Plan::new(vec![entry]);

    let result = plan_event(plan);
    let entries = result.get("entries").unwrap();

    let mut expected_entry = nvim_oxi::Dictionary::new();
    expected_entry.insert("content", "Task");
    expected_entry.insert("priority", "Medium");
    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_entry)]);
    assert_eq!(*entries, nvim_oxi::Object::from(expected));
}

#[test]
fn test_plan_event_entry_priority_low() {
    let entry = PlanEntry::new("Task", PlanEntryPriority::Low, PlanEntryStatus::Pending);
    let plan = Plan::new(vec![entry]);

    let result = plan_event(plan);
    let entries = result.get("entries").unwrap();

    let mut expected_entry = nvim_oxi::Dictionary::new();
    expected_entry.insert("content", "Task");
    expected_entry.insert("priority", "Low");
    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_entry)]);
    assert_eq!(*entries, nvim_oxi::Object::from(expected));
}

#[test]
fn test_plan_event_without_meta() {
    let entry = PlanEntry::new("Task", PlanEntryPriority::High, PlanEntryStatus::Pending);
    let plan = Plan::new(vec![entry]);

    let result = plan_event(plan);
    assert_eq!(result.get("meta").is_some(), false);
}

#[test]
fn test_plan_event_with_meta() {
    let entry = PlanEntry::new("Task", PlanEntryPriority::High, PlanEntryStatus::Pending);
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "llm"})
        .as_object()
        .unwrap()
        .clone();
    let plan = Plan::new(vec![entry]).meta(meta);

    let result = plan_event(plan);
    assert_eq!(result.get("meta").is_some(), true);
}
