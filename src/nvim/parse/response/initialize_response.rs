use agent_client_protocol::InitializeResponse;
use nvim_oxi::Dictionary;

pub fn initialize_response(response: InitializeResponse) -> Dictionary {
    let mut data = nvim_oxi::Dictionary::new();

    data.insert("protocolVersion", response.protocol_version.to_string());

    let mut capabilities_dict = nvim_oxi::Dictionary::new();
    capabilities_dict.insert("loadSession", response.agent_capabilities.load_session);

    let mut prompt_caps_dict = nvim_oxi::Dictionary::new();
    prompt_caps_dict.insert(
        "image",
        response.agent_capabilities.prompt_capabilities.image,
    );
    prompt_caps_dict.insert(
        "audio",
        response.agent_capabilities.prompt_capabilities.audio,
    );
    prompt_caps_dict.insert(
        "embeddedContext",
        response
            .agent_capabilities
            .prompt_capabilities
            .embedded_context,
    );
    capabilities_dict.insert("promptCapabilities", prompt_caps_dict);

    let mut mcp_caps_dict = nvim_oxi::Dictionary::new();
    mcp_caps_dict.insert("http", response.agent_capabilities.mcp_capabilities.http);
    mcp_caps_dict.insert("sse", response.agent_capabilities.mcp_capabilities.sse);
    capabilities_dict.insert("mcpCapabilities", mcp_caps_dict);

    let mut session_caps_dict = nvim_oxi::Dictionary::new();
    if response
        .agent_capabilities
        .session_capabilities
        .list
        .is_some()
    {
        session_caps_dict.insert("list", true);
    }
    if response
        .agent_capabilities
        .session_capabilities
        .fork
        .is_some()
    {
        session_caps_dict.insert("fork", true);
    }
    if response
        .agent_capabilities
        .session_capabilities
        .resume
        .is_some()
    {
        session_caps_dict.insert("resume", true);
    }
    capabilities_dict.insert("sessionCapabilities", session_caps_dict);

    data.insert("agentCapabilities", capabilities_dict);

    let auth_methods_arr =
        nvim_oxi::Array::from_iter(response.auth_methods.into_iter().map(|method| {
            let mut method_dict = nvim_oxi::Dictionary::new();
            method_dict.insert("id", method.id.0.as_ref().to_string());
            method_dict.insert("name", method.name.as_str());
            if let Some(description) = method.description {
                method_dict.insert("description", description);
            }
            method_dict
        }));
    data.insert("authMethods", auth_methods_arr);

    if let Some(agent_info) = response.agent_info {
        let mut info_dict = nvim_oxi::Dictionary::new();
        info_dict.insert("name", agent_info.name.as_str());
        info_dict.insert("version", agent_info.version.as_str());
        if let Some(title) = agent_info.title {
            info_dict.insert("title", title);
        }
        data.insert("agentInfo", info_dict);
    }

    if let Some(meta) = crate::nvim::parse::convert_metadata_to_lua_object(response.meta) {
        data.insert("meta", meta);
    }

    data
}
