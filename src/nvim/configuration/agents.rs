use std::collections::HashMap;

use nvim_oxi::{
    Dictionary, Object,
    conversion::{Error, FromObject},
    lua,
};
use serde::{Deserialize, Serialize};

use crate::{
    acp::connection::{Assistant, Protocol},
    api::AgentListEntry,
};

use super::dict_from_object;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AgentConnectionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub protocol: Protocol,
    pub connection: Assistant,
}

impl FromObject for AgentConnectionConfig {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj.clone())?;

        let protocol = dict
            .get("protocol")
            .and_then(|o| {
                o.clone()
                    .try_into()
                    .ok()
                    .map(|value: nvim_oxi::String| Protocol::from(value.to_string()))
            })
            .unwrap_or_default();

        let connection = Assistant::from_object(obj)?;

        Ok(Self {
            description: optional_string(&dict, "description"),
            icon: optional_string(&dict, "icon"),
            protocol,
            connection,
        })
    }
}

fn optional_string(dict: &Dictionary, key: &str) -> Option<String> {
    dict.get(key)
        .and_then(|obj| obj.clone().try_into().ok())
        .map(|value: nvim_oxi::String| value.to_string())
}

impl From<AgentConnectionConfig> for Dictionary {
    fn from(config: AgentConnectionConfig) -> Self {
        let mut dict = config.connection.to_connection_dict(config.protocol);
        if let Some(description) = config.description {
            dict.insert("description", description);
        }
        if let Some(icon) = config.icon {
            dict.insert("icon", icon);
        }
        dict
    }
}

impl lua::Pushable for AgentConnectionConfig {
    unsafe fn push(self, lua_state: *mut lua::ffi::State) -> Result<i32, lua::Error> {
        unsafe { Dictionary::from(self).push(lua_state) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CustomAgentsConfig(pub HashMap<String, AgentConnectionConfig>);

impl CustomAgentsConfig {
    pub fn get(&self, id: &str) -> Option<&AgentConnectionConfig> {
        self.0.get(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<AgentConnectionConfig> {
        self.0.remove(id)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn apply_to(self, config: &mut CustomAgentsConfig) {
        for (id, entry) in self.0 {
            config.0.insert(id, entry);
        }
    }
}

impl Into<Vec<AgentListEntry>> for CustomAgentsConfig {
    fn into(self) -> Vec<AgentListEntry> {
        self.0
            .iter()
            .map(|(id, entry)| AgentListEntry::from_custom(id, entry))
            .collect::<Vec<AgentListEntry>>()
    }
}

impl FromObject for CustomAgentsConfig {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;

        let mut agents = HashMap::new();
        for (key, value) in dict {
            let id = key.to_string();
            let entry = AgentConnectionConfig::from_object(value)?;
            agents.insert(id, entry);
        }

        Ok(Self(agents))
    }
}

impl From<CustomAgentsConfig> for Dictionary {
    fn from(config: CustomAgentsConfig) -> Self {
        let mut dict = Dictionary::new();
        for (id, entry) in config.0 {
            dict.insert(id, Dictionary::from(entry));
        }
        dict
    }
}

impl lua::Pushable for CustomAgentsConfig {
    unsafe fn push(self, lua_state: *mut lua::ffi::State) -> Result<i32, lua::Error> {
        unsafe { Dictionary::from(self).push(lua_state) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::connection::Protocol;
    use nvim_oxi::Array;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    fn stdio_config() -> AgentConnectionConfig {
        AgentConnectionConfig {
            description: None,
            icon: None,
            protocol: Protocol::Stdio,
            connection: Assistant::CustomStdio {
                name: String::new(),
                command: "my-agent".to_string(),
                args: vec!["acp".to_string()],
            },
        }
    }

    #[test]
    fn agent_connection_config_parses_stdio() {
        let mut dict = Dictionary::new();
        dict.insert("command", "my-agent");
        dict.insert("args", Array::from_iter(["acp", "--flag"]));

        let config = AgentConnectionConfig::from_object(Object::from(dict)).expect("parses");

        assert_eq!(
            config.connection,
            Assistant::CustomStdio {
                name: String::new(),
                command: "my-agent".to_string(),
                args: vec!["acp".to_string(), "--flag".to_string()],
            }
        );
    }

    #[test]
    fn agent_connection_config_parses_name_into_stdio() {
        let mut dict = Dictionary::new();
        dict.insert("name", "My Agent");
        dict.insert("command", "my-agent");

        let config = AgentConnectionConfig::from_object(Object::from(dict)).expect("parses");

        assert_eq!(
            config.connection,
            Assistant::CustomStdio {
                name: "My Agent".to_string(),
                command: "my-agent".to_string(),
                args: Vec::new(),
            }
        );
    }

    #[test]
    fn agent_connection_config_parses_description_and_icon() {
        let mut dict = Dictionary::new();
        dict.insert("description", "does things");
        dict.insert("icon", "~/icons/my.svg");
        dict.insert("command", "my-agent");

        let config = AgentConnectionConfig::from_object(Object::from(dict)).expect("parses");

        assert_eq!(
            (config.description.as_deref(), config.icon.as_deref()),
            (Some("does things"), Some("~/icons/my.svg"))
        );
    }

    #[test]
    fn agent_connection_config_parses_socket() {
        let mut dict = Dictionary::new();
        dict.insert("protocol", "socket");
        dict.insert("path", "/tmp/agent.sock");

        let config = AgentConnectionConfig::from_object(Object::from(dict)).expect("parses");

        assert_eq!(
            config.connection,
            Assistant::CustomSocket {
                name: String::new(),
                path: "/tmp/agent.sock".to_string(),
            }
        );
    }

    #[test]
    fn agent_connection_config_parses_url() {
        let mut dict = Dictionary::new();
        dict.insert("protocol", "http");
        dict.insert("host", "localhost");
        dict.insert("port", 8080i64);
        dict.insert("path", "/acp");

        let config = AgentConnectionConfig::from_object(Object::from(dict)).expect("parses");

        assert_eq!(
            config.connection,
            Assistant::CustomUrl {
                name: String::new(),
                host: "localhost".to_string(),
                port: 8080,
                path: Some("/acp".to_string()),
            }
        );
    }

    #[test]
    fn agent_connection_config_stdio_without_command_is_error() {
        let dict = Dictionary::new();

        let result = AgentConnectionConfig::from_object(Object::from(dict));

        assert!(result.is_err());
    }

    #[test]
    fn agent_connection_config_socket_without_path_is_error() {
        let mut dict = Dictionary::new();
        dict.insert("protocol", "socket");

        let result = AgentConnectionConfig::from_object(Object::from(dict));

        assert!(result.is_err());
    }

    #[test]
    fn agent_connection_config_url_without_port_is_error() {
        let mut dict = Dictionary::new();
        dict.insert("protocol", "tcp");
        dict.insert("host", "localhost");

        let result = AgentConnectionConfig::from_object(Object::from(dict));

        assert!(result.is_err());
    }

    #[test]
    fn agent_connection_config_round_trips_stdio() {
        let config = stdio_config();

        let reparsed =
            AgentConnectionConfig::from_object(Object::from(Dictionary::from(config.clone())))
                .expect("parses");

        assert_eq!(reparsed, config);
    }

    #[test]
    fn agent_connection_config_round_trips_url_protocol() {
        let config = AgentConnectionConfig {
            description: None,
            icon: None,
            protocol: Protocol::Https,
            connection: Assistant::CustomUrl {
                name: String::new(),
                host: "example.com".to_string(),
                port: 443,
                path: Some("/acp".to_string()),
            },
        };

        let reparsed =
            AgentConnectionConfig::from_object(Object::from(Dictionary::from(config.clone())))
                .expect("parses");

        assert_eq!(reparsed, config);
    }

    #[test]
    fn custom_agents_config_parses_entries() {
        let mut entry = Dictionary::new();
        entry.insert("command", "opencode");
        let mut dict = Dictionary::new();
        dict.insert("opencode", entry);

        let config = CustomAgentsConfig::from_object(Object::from(dict)).expect("parses");

        assert_eq!(
            config.get("opencode"),
            Some(&stdio_config_named("opencode"))
        );
    }

    fn stdio_config_named(command: &str) -> AgentConnectionConfig {
        AgentConnectionConfig {
            description: None,
            icon: None,
            protocol: Protocol::Stdio,
            connection: Assistant::CustomStdio {
                name: String::new(),
                command: command.to_string(),
                args: Vec::new(),
            },
        }
    }

    #[test]
    fn custom_agents_config_empty_dict_is_empty() {
        let config =
            CustomAgentsConfig::from_object(Object::from(Dictionary::new())).expect("parses");
        assert!(config.is_empty());
    }

    #[test]
    fn custom_agents_config_empty_array_is_empty() {
        let config = CustomAgentsConfig::from_object(Object::from(Array::new())).expect("parses");
        assert!(config.is_empty());
    }

    #[test]
    fn custom_agents_config_apply_to_merges_entries() {
        let mut base = CustomAgentsConfig(HashMap::from([(
            "existing".to_string(),
            stdio_config_named("existing"),
        )]));
        let incoming = CustomAgentsConfig(HashMap::from([(
            "new".to_string(),
            stdio_config_named("new"),
        )]));

        incoming.apply_to(&mut base);

        assert_eq!(base.0.len(), 2);
    }

    #[test]
    fn custom_agents_config_apply_to_overwrites_duplicate_id() {
        let mut base = CustomAgentsConfig(HashMap::from([(
            "agent".to_string(),
            stdio_config_named("original"),
        )]));
        let incoming = CustomAgentsConfig(HashMap::from([(
            "agent".to_string(),
            stdio_config_named("replaced"),
        )]));

        incoming.apply_to(&mut base);

        assert_eq!(base.get("agent"), Some(&stdio_config_named("replaced")));
    }

    proptest! {
        #[test]
        fn agent_connection_config_from_object_never_panics(command in "[a-zA-Z0-9_/ .-]*") {
            let mut dict = Dictionary::new();
            dict.insert("command", command);
            let _ = AgentConnectionConfig::from_object(Object::from(dict));
        }

        #[test]
        fn custom_agents_config_from_object_never_panics(id in "[a-zA-Z0-9_-]*") {
            let mut dict = Dictionary::new();
            dict.insert(id, Dictionary::new());
            let _ = CustomAgentsConfig::from_object(Object::from(dict));
        }
    }
}
