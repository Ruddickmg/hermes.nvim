use agent_client_protocol::SessionMode;

pub fn parse_modes(modes: Vec<SessionMode>) -> nvim_oxi::Array {
    nvim_oxi::Array::from_iter(modes.into_iter().map(|mode| {
        let mut mode_dict = nvim_oxi::Dictionary::new();
        mode_dict.insert("id", mode.id.0.as_ref().to_string());
        mode_dict.insert("name", mode.name.as_str());
        if let Some(description) = mode.description {
            mode_dict.insert("description", description);
        }
        mode_dict
    }))
}
