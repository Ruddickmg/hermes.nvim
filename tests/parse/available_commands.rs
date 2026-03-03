use agent_client_protocol::{
    AvailableCommand, AvailableCommandInput, AvailableCommandsUpdate, UnstructuredCommandInput,
};
use hermes::nvim::parse::available_commands_event;

#[test]
fn test_available_commands_event_ok() {
    let cmd = AvailableCommand::new("read_file", "Read a file");
    let update = AvailableCommandsUpdate::new(vec![cmd]);

    let result = available_commands_event(update);
    let commands = result.get("commands").unwrap();
    let mut expected_cmd = nvim_oxi::Dictionary::new();
    expected_cmd.insert("name", "read_file");
    expected_cmd.insert("description", "Read a file");
    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_cmd)]);
    assert_eq!(*commands, nvim_oxi::Object::from(expected));
}

#[test]
fn test_available_commands_event_empty_array() {
    let update = AvailableCommandsUpdate::new(vec![]);

    let result = available_commands_event(update);
    let commands = result.get("commands").unwrap();
    assert_eq!(*commands, nvim_oxi::Object::from(nvim_oxi::Array::new()));
}

#[test]
fn test_available_commands_event_single_command() {
    let cmd = AvailableCommand::new("read_file", "Read a file");
    let update = AvailableCommandsUpdate::new(vec![cmd]);

    let result = available_commands_event(update);
    let commands = result.get("commands").unwrap();

    let mut expected_cmd = nvim_oxi::Dictionary::new();
    expected_cmd.insert("name", "read_file");
    expected_cmd.insert("description", "Read a file");
    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_cmd)]);

    assert_eq!(*commands, nvim_oxi::Object::from(expected));
}

#[test]
fn test_available_commands_event_multiple_commands() {
    let cmd1 = AvailableCommand::new("read_file", "Read a file");
    let cmd2 = AvailableCommand::new("write_file", "Write a file");
    let update = AvailableCommandsUpdate::new(vec![cmd1, cmd2]);

    let result = available_commands_event(update);
    let commands = result.get("commands").unwrap();

    let mut expected_cmd1 = nvim_oxi::Dictionary::new();
    expected_cmd1.insert("name", "read_file");
    expected_cmd1.insert("description", "Read a file");

    let mut expected_cmd2 = nvim_oxi::Dictionary::new();
    expected_cmd2.insert("name", "write_file");
    expected_cmd2.insert("description", "Write a file");

    let expected = nvim_oxi::Array::from_iter([
        nvim_oxi::Object::from(expected_cmd1),
        nvim_oxi::Object::from(expected_cmd2),
    ]);

    assert_eq!(*commands, nvim_oxi::Object::from(expected));
}

#[test]
fn test_available_commands_event_without_input() {
    let cmd = AvailableCommand::new("delete_file", "Delete a file");
    let update = AvailableCommandsUpdate::new(vec![cmd]);

    let result = available_commands_event(update);
    let commands = result.get("commands").unwrap();

    let mut expected_cmd = nvim_oxi::Dictionary::new();
    expected_cmd.insert("name", "delete_file");
    expected_cmd.insert("description", "Delete a file");
    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_cmd)]);

    assert_eq!(*commands, nvim_oxi::Object::from(expected));
}

#[test]
fn test_available_commands_event_with_input() {
    let cmd = AvailableCommand::new("search", "Search for text").input(
        AvailableCommandInput::Unstructured(UnstructuredCommandInput::new("Enter search query...")),
    );
    let update = AvailableCommandsUpdate::new(vec![cmd]);

    let result = available_commands_event(update);
    let commands = result.get("commands").unwrap();

    let mut expected_cmd = nvim_oxi::Dictionary::new();
    expected_cmd.insert("name", "search");
    expected_cmd.insert("description", "Search for text");

    let mut input_dict = nvim_oxi::Dictionary::new();
    input_dict.insert("hint", "Enter search query...");
    expected_cmd.insert("input", input_dict);

    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_cmd)]);

    assert_eq!(*commands, nvim_oxi::Object::from(expected));
}

#[test]
fn test_available_commands_event_without_meta() {
    let cmd = AvailableCommand::new("list_files", "List files");
    let update = AvailableCommandsUpdate::new(vec![cmd]);

    let result = available_commands_event(update);
    assert_eq!(result.get("meta").is_some(), false);
}

#[test]
fn test_available_commands_event_with_meta() {
    let cmd = AvailableCommand::new("edit_file", "Edit a file");
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "agent"})
        .as_object()
        .unwrap()
        .clone();
    let update = AvailableCommandsUpdate::new(vec![cmd]).meta(meta);

    let result = available_commands_event(update);
    let meta_obj = result.get("meta").unwrap();
    let mut expected_meta = nvim_oxi::Dictionary::new();
    expected_meta.insert("source", "agent");
    assert_eq!(*meta_obj, nvim_oxi::Object::from(expected_meta));
}
