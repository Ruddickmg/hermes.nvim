use agent_client_protocol::AvailableCommandInput;
use agent_client_protocol::AvailableCommandsUpdate;
use nvim_oxi::Dictionary;

pub fn available_commands_event(update: AvailableCommandsUpdate) -> Dictionary {
    let mut data: nvim_oxi::Dictionary = nvim_oxi::Dictionary::new();
    let commands = update.available_commands.into_iter().map(|command| {
        let mut dict = nvim_oxi::Dictionary::new();
        dict.insert("name", command.name);
        dict.insert("description", command.description);
        if let Some(AvailableCommandInput::Unstructured(input)) = command.input {
            dict.insert(
                "input",
                Dictionary::from_iter(vec![
                    ("hint", input.hint),
                    // TODO: get meta figured out
                    // ("meta", input.meta)
                ]),
            );
        }
        dict
    });
    data.insert("commands", nvim_oxi::Array::from_iter(commands));

    if let Some(meta) = update.meta {
        data.insert("meta", format!("{:?}", meta));
    }

    data
}
