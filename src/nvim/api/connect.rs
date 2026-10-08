use crate::{
    acp::{
        Result,
        connection::{Assistant, ConnectionDetails, Protocol},
        error::Error,
        registry::distribution::Distribution,
    },
    api::Api,
    nvim::configuration::dict_from_object,
};
use nvim_oxi::{
    Dictionary, Object, ObjectKind,
    conversion::FromObject,
    lua::{self, Error as LuaError, Poppable, Pushable},
};
use tracing::error;

#[derive(Clone, Debug, Default)]
pub struct ConnectionOptions {
    protocol: Protocol,
    distribution: Option<Distribution>,
    command: Option<String>,
    args: Option<Vec<String>>,
    host: Option<String>,
    port: Option<u16>,
    path: Option<String>,
}

impl ConnectionOptions {
    fn into_assistant(self, name: String) -> Result<Assistant> {
        match self.protocol {
            Protocol::Stdio => match self.command {
                Some(command) => {
                    if command.is_empty() {
                        return Err(Error::InvalidInput(
                            "Command cannot be empty for custom stdio connections".into(),
                        ));
                    }
                    Ok(Assistant::CustomStdio {
                        name,
                        command,
                        args: self.args.unwrap_or_default(),
                    })
                }
                None => {
                    let assistant = Assistant::from(name);
                    if matches!(assistant, Assistant::CustomStdio { .. }) {
                        return Err(Error::InvalidInput(
                            "Custom stdio connections require options with a 'command' field"
                                .into(),
                        ));
                    }
                    Ok(assistant)
                }
            },
            Protocol::Socket => match self.path {
                Some(path) => Ok(Assistant::CustomSocket { name, path }),
                None => Err(Error::InvalidInput(
                    "Path must be provided for socket connections".into(),
                )),
            },
            _ => match (self.host, self.port) {
                (Some(host), Some(port)) => Ok(Assistant::CustomUrl {
                    name,
                    host,
                    port,
                    path: self.path,
                }),
                _ => Err(Error::InvalidInput(format!(
                    "Host and port must be provided for {} connections",
                    self.protocol
                ))),
            },
        }
    }
}

impl FromObject for ConnectionOptions {
    fn from_object(obj: Object) -> std::result::Result<Self, nvim_oxi::conversion::Error> {
        if obj.is_nil() {
            return Ok(Self::default());
        }

        let dict = dict_from_object(obj)?;

        let protocol: Protocol = dict
            .get("protocol")
            .and_then(|o| {
                o.clone()
                    .try_into()
                    .ok()
                    .map(|s: nvim_oxi::String| Protocol::from(s.to_string()))
            })
            .unwrap_or_default();

        let distribution: Option<Distribution> = match dict.get("distribution") {
            Some(obj) => {
                let s: nvim_oxi::String = obj.clone().try_into().map_err(|_| {
                    nvim_oxi::conversion::Error::FromWrongType {
                        expected: "string",
                        actual: obj.kind().as_static(),
                    }
                })?;
                let lower = s.to_string().to_lowercase();
                match Distribution::from(lower) {
                    Distribution::Invalid => {
                        return Err(nvim_oxi::conversion::Error::FromWrongType {
                            expected: "one of: npx, uvx, binary",
                            actual: obj.kind().as_static(),
                        });
                    }
                    valid => Some(valid),
                }
            }
            None => None,
        };

        let command: Option<String> = dict.get("command").and_then(|o| {
            o.clone()
                .try_into()
                .ok()
                .map(|s: nvim_oxi::String| s.to_string())
        });

        let args: Option<Vec<String>> = dict.get("args").and_then(|o| {
            if o.kind() == ObjectKind::Array {
                let arr: nvim_oxi::Array = unsafe { o.clone().into_array_unchecked() };
                Some(
                    arr.into_iter()
                        .filter_map(|v| v.try_into().ok().map(|s: nvim_oxi::String| s.to_string()))
                        .collect(),
                )
            } else {
                None
            }
        });

        let host: Option<String> = dict.get("host").and_then(|o| {
            o.clone()
                .try_into()
                .ok()
                .map(|s: nvim_oxi::String| s.to_string())
        });

        let port: Option<u16> = dict.get("port").and_then(|o| o.clone().try_into().ok());

        let path: Option<String> = dict.get("path").and_then(|o| {
            o.clone()
                .try_into()
                .ok()
                .map(|s: nvim_oxi::String| s.to_string())
        });

        Ok(Self {
            protocol,
            distribution,
            command,
            args,
            host,
            port,
            path,
        })
    }
}

impl Poppable for ConnectionOptions {
    unsafe fn pop(lua_state: *mut lua::ffi::State) -> std::result::Result<Self, LuaError> {
        let obj = unsafe { Object::pop(lua_state)? };
        Ok(Self::from_object(obj)
            .inspect_err(|e| error!("Error occurred while parsing connection options: {:?}", e))
            .unwrap_or_default())
    }
}

impl Pushable for ConnectionOptions {
    unsafe fn push(self, lua_state: *mut lua::ffi::State) -> std::result::Result<i32, LuaError> {
        let mut dict = Dictionary::new();
        dict.insert("protocol", self.protocol.to_string());
        if let Some(distribution) = self.distribution {
            dict.insert("distribution", distribution.to_string());
        }
        if let Some(command) = self.command {
            dict.insert("command", command);
        }
        if let Some(args) = self.args {
            let arr: nvim_oxi::Array = args.into_iter().collect();
            dict.insert("args", arr);
        }
        if let Some(host) = self.host {
            dict.insert("host", host);
        }
        if let Some(port) = self.port {
            dict.insert("port", port);
        }
        if let Some(path) = self.path {
            dict.insert("path", path);
        }
        unsafe { Object::from(dict).push(lua_state) }
    }
}

pub type ConnectionArgs = (nvim_oxi::String, Option<ConnectionOptions>);

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn connect(&mut self, (agent_name, options): ConnectionArgs) -> Result<()> {
        let opts = options.unwrap_or_default();
        let agent_id = agent_name.to_string();
        let state = self.state.lock().await;
        let registry = state.registry.clone();
        let distribution_config = state.config.distributions.clone();
        drop(state);

        let agent = if let Some(Some(entry)) = registry
            .as_ref()
            .map(|r| r.data.get_entry(&agent_id).cloned())
        {
            Assistant::Registered {
                agent: entry,
                configuration: distribution_config,
                distribution: opts.distribution,
                command: opts.command,
                args: opts.args,
                registry: registry.clone(),
            }
        } else {
            opts.clone().into_assistant(agent_id)?
        };

        let handler = self.response_handler.clone();
        let mut connection_manager = self.connection_manager.lock().await;
        let result = connection_manager
            .connect(
                handler,
                ConnectionDetails {
                    agent,
                    protocol: opts.protocol,
                },
            )
            .await
            .map(|_| ());
        drop(connection_manager);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    // Strategy for generating agent names
    fn arb_agent_name() -> impl Strategy<Value = String> {
        prop_oneof!(
            Just("copilot".to_string()),
            Just("opencode".to_string()),
            Just("COPILOT".to_string()),
            Just("OPENCODE".to_string()),
            Just("Copilot".to_string()),
            Just("Opencode".to_string()),
            "[a-zA-Z][a-zA-Z0-9_-]*".prop_map(|s| s.to_string())
        )
    }

    proptest! {
        #[test]
        fn test_assistant_from_str_never_panics(name in arb_agent_name()) {
            // Property: converting any string to Assistant should never panic
            let _ = Assistant::from(name.as_str());
        }

        #[test]
        fn test_known_agents_parsed_correctly(name in prop_oneof!(
            Just("copilot"),
            Just("opencode"),
            Just("COPILOT"),
            Just("OPENCODE")
        )) {
            // Property: Known agents should parse to their respective variants
            let assistant = Assistant::from(name);
            let name_lower = name.to_lowercase();

            if name_lower == "copilot" {
                prop_assert!(matches!(assistant, Assistant::Copilot), "Expected Copilot variant");
            } else if name_lower == "opencode" {
                prop_assert!(matches!(assistant, Assistant::Opencode), "Expected Opencode variant");
            }
        }
    }

    #[test]
    fn test_assistant_from_str_copilot() {
        assert!(matches!(Assistant::from("copilot"), Assistant::Copilot));
    }

    #[test]
    fn test_assistant_from_str_opencode() {
        assert!(matches!(Assistant::from("opencode"), Assistant::Opencode));
    }

    #[test]
    fn test_assistant_from_str_case_insensitive() {
        assert!(matches!(Assistant::from("COPILOT"), Assistant::Copilot));
        assert!(matches!(Assistant::from("OpEnCoDe"), Assistant::Opencode));
    }

    #[test]
    fn test_assistant_custom_with_command() {
        let assistant = Assistant::from("custom-agent");
        assert!(matches!(assistant, Assistant::CustomStdio { name, .. } if name == "custom-agent"));
    }

    // Tests for ConnectionOptions into_assistant
    #[test]
    fn connection_options_into_assistant_copilot() {
        let opts = ConnectionOptions::default();
        let result = opts.into_assistant("copilot".to_string());
        assert!(matches!(result.unwrap(), Assistant::Copilot));
    }

    #[test]
    fn connection_options_into_assistant_opencode() {
        let opts = ConnectionOptions::default();
        let result = opts.into_assistant("opencode".to_string());
        assert!(matches!(result.unwrap(), Assistant::Opencode));
    }

    #[test]
    fn connection_options_into_assistant_custom_stdio() {
        let opts = ConnectionOptions {
            command: Some("my-agent".to_string()),
            args: Some(vec!["arg1".to_string(), "arg2".to_string()]),
            ..Default::default()
        };
        let result = opts.into_assistant("test-agent".to_string());
        let assistant = result.unwrap();
        assert!(
            matches!(assistant, Assistant::CustomStdio { name, command, args } 
            if name == "test-agent" && command == "my-agent" && args == vec!["arg1", "arg2"])
        );
    }

    #[test]
    fn connection_options_into_assistant_http_with_host_port() {
        let opts = ConnectionOptions {
            protocol: Protocol::Http,
            host: Some("api.example.com".to_string()),
            port: Some(443),
            ..Default::default()
        };
        let result = opts.into_assistant("http-agent".to_string());
        let assistant = result.unwrap();
        assert!(
            matches!(assistant, Assistant::CustomUrl { name: _, host, port, path: _ }
            if host == "api.example.com" && port == 443)
        );
    }

    #[test]
    fn connection_options_into_assistant_https_with_host_port() {
        let opts = ConnectionOptions {
            protocol: Protocol::Https,
            host: Some("api.example.com".to_string()),
            port: Some(443),
            ..Default::default()
        };
        let result = opts.into_assistant("https-agent".to_string());
        let assistant = result.unwrap();
        assert!(
            matches!(assistant, Assistant::CustomUrl { name: _, host, port, path: _ }
            if host == "api.example.com" && port == 443)
        );
    }

    #[test]
    fn connection_options_into_assistant_socket_with_path() {
        let opts = ConnectionOptions {
            protocol: Protocol::Socket,
            path: Some("/tmp/agent.sock".to_string()),
            ..Default::default()
        };
        let result = opts.into_assistant("socket-agent".to_string());
        let assistant = result.unwrap();
        assert!(matches!(assistant, Assistant::CustomSocket { name, path }
            if name == "socket-agent" && path == "/tmp/agent.sock"));
    }

    #[test]
    fn connection_options_into_assistant_socket_missing_path() {
        let opts = ConnectionOptions {
            protocol: Protocol::Socket,
            ..Default::default()
        };
        let result = opts.into_assistant("socket-agent".to_string());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Path must be provided"));
    }

    #[test]
    fn connection_options_into_assistant_tcp_with_host_port() {
        let opts = ConnectionOptions {
            protocol: Protocol::Tcp,
            host: Some("tcp.example.com".to_string()),
            port: Some(9090),
            ..Default::default()
        };
        let result = opts.into_assistant("tcp-agent".to_string());
        let assistant = result.unwrap();
        assert!(
            matches!(assistant, Assistant::CustomUrl { name, host, port, path: _ }
            if name == "tcp-agent" && host == "tcp.example.com" && port == 9090)
        );
    }

    #[test]
    fn connection_options_into_assistant_socket_missing_port() {
        let opts = ConnectionOptions {
            protocol: Protocol::Tcp,
            host: Some("localhost".to_string()),
            ..Default::default()
        };
        let result = opts.into_assistant("tcp-agent".to_string());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Host and port must be provided")
        );
    }

    #[test]
    fn connection_options_into_assistant_empty_command() {
        let opts = ConnectionOptions {
            command: Some(String::new()),
            ..Default::default()
        };
        let result = opts.into_assistant("test-agent".to_string());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Command cannot be empty")
        );
    }

    #[test]
    fn connection_options_into_assistant_custom_stdio_without_command() {
        let opts = ConnectionOptions::default();
        let result = opts.into_assistant("custom-agent".to_string());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("require options with a 'command' field")
        );
    }

    // Tests for ConnectionOptions FromObject
    #[test]
    fn connection_options_from_object_nil() {
        let result = ConnectionOptions::from_object(Object::nil());
        assert!(result.unwrap().protocol == Protocol::Stdio);
    }

    #[test]
    fn connection_options_from_object_protocol() {
        let mut dict = Dictionary::new();
        dict.insert("protocol", "socket");
        let result = ConnectionOptions::from_object(Object::from(dict));
        assert_eq!(result.unwrap().protocol, Protocol::Socket);
    }

    #[test]
    fn connection_options_from_object_protocol_https() {
        let mut dict = Dictionary::new();
        dict.insert("protocol", "https");
        let result = ConnectionOptions::from_object(Object::from(dict));
        assert_eq!(result.unwrap().protocol, Protocol::Https);
    }

    #[test]
    fn connection_options_from_object_distribution_npx() {
        let mut dict = Dictionary::new();
        dict.insert("distribution", "npx");
        let result = ConnectionOptions::from_object(Object::from(dict));
        assert_eq!(result.unwrap().distribution, Some(Distribution::Npx));
    }

    #[test]
    fn connection_options_from_object_distribution_uvx() {
        let mut dict = Dictionary::new();
        dict.insert("distribution", "uvx");
        let result = ConnectionOptions::from_object(Object::from(dict));
        assert_eq!(result.unwrap().distribution, Some(Distribution::Uvx));
    }

    #[test]
    fn connection_options_from_object_distribution_binary() {
        let mut dict = Dictionary::new();
        dict.insert("distribution", "binary");
        let result = ConnectionOptions::from_object(Object::from(dict));
        assert_eq!(result.unwrap().distribution, Some(Distribution::Binary));
    }

    #[test]
    fn connection_options_from_object_distribution_case_insensitive() {
        let mut dict = Dictionary::new();
        dict.insert("distribution", "NPX");
        let result = ConnectionOptions::from_object(Object::from(dict));
        assert_eq!(result.unwrap().distribution, Some(Distribution::Npx));
    }

    #[test]
    fn connection_options_from_object_distribution_missing() {
        let dict = Dictionary::new();
        let result = ConnectionOptions::from_object(Object::from(dict));
        assert!(result.unwrap().distribution.is_none());
    }

    #[test]
    fn connection_options_from_object_distribution_invalid() {
        let mut dict = Dictionary::new();
        dict.insert("distribution", "bad");
        let result = ConnectionOptions::from_object(Object::from(dict));
        assert!(result.is_err());
    }

    #[test]
    fn connection_options_from_object_distribution_non_string() {
        let mut dict = Dictionary::new();
        dict.insert("distribution", 42i64);
        let result = ConnectionOptions::from_object(Object::from(dict));
        assert!(result.is_err());
    }

    #[test]
    fn connection_options_from_object_with_command_and_args() {
        let mut dict = Dictionary::new();
        dict.insert("command", "my-agent");
        dict.insert("args", nvim_oxi::Array::from_iter(["arg1", "arg2"]));
        let result = ConnectionOptions::from_object(Object::from(dict));
        let opts = result.unwrap();
        assert_eq!(opts.command.as_deref(), Some("my-agent"));
    }

    #[test]
    fn connection_options_from_object_args_non_array_sets_args_to_none() {
        let mut dict = Dictionary::new();
        dict.insert("command", "my-agent");
        dict.insert("args", "not-an-array");
        let result = ConnectionOptions::from_object(Object::from(dict));
        let opts = result.unwrap();
        assert!(opts.args.is_none());
    }

    #[test]
    fn connection_options_from_object_with_host_port() {
        let mut dict = Dictionary::new();
        dict.insert("host", "localhost");
        dict.insert("port", 8080i64);
        let result = ConnectionOptions::from_object(Object::from(dict));
        let opts = result.unwrap();
        assert_eq!(opts.host.as_deref(), Some("localhost"));
    }

    #[test]
    fn connection_options_from_object_with_path() {
        let mut dict = Dictionary::new();
        dict.insert("protocol", "http");
        dict.insert("host", "localhost");
        dict.insert("port", 8080i64);
        dict.insert("path", "/v1/acp");
        let result = ConnectionOptions::from_object(Object::from(dict));
        let opts = result.unwrap();
        assert_eq!(opts.path.as_deref(), Some("/v1/acp"));
    }

    #[test]
    fn connection_options_from_object_protocol_defaults_to_stdio() {
        let dict = Dictionary::new();
        let result = ConnectionOptions::from_object(Object::from(dict));
        assert_eq!(result.unwrap().protocol, Protocol::Stdio);
    }

    // Tests for Protocol parsing
    #[test]
    fn test_protocol_from_str_stdio() {
        assert!(matches!(Protocol::from("stdio"), Protocol::Stdio));
        assert!(matches!(Protocol::from("STDIO"), Protocol::Stdio));
        assert!(matches!(Protocol::from("Stdio"), Protocol::Stdio));
    }

    #[test]
    fn test_protocol_from_str_socket() {
        assert!(matches!(Protocol::from("socket"), Protocol::Socket));
        assert!(matches!(Protocol::from("SOCKET"), Protocol::Socket));
    }

    #[test]
    fn test_protocol_from_str_http() {
        assert!(matches!(Protocol::from("http"), Protocol::Http));
        assert!(matches!(Protocol::from("HTTP"), Protocol::Http));
    }

    #[test]
    fn test_protocol_from_str_https() {
        assert!(matches!(Protocol::from("https"), Protocol::Https));
        assert!(matches!(Protocol::from("HTTPS"), Protocol::Https));
    }

    #[test]
    fn test_protocol_from_str_unknown_defaults_to_stdio() {
        assert!(matches!(Protocol::from("unknown"), Protocol::Stdio));
        assert!(matches!(Protocol::from(""), Protocol::Stdio));
    }

    // Tests for Protocol Display trait
    #[test]
    fn test_protocol_display_stdio() {
        assert_eq!(format!("{}", Protocol::Stdio), "stdio");
    }

    #[test]
    fn test_protocol_display_socket() {
        assert_eq!(format!("{}", Protocol::Socket), "socket");
    }

    #[test]
    fn test_protocol_display_http() {
        assert_eq!(format!("{}", Protocol::Http), "http");
    }

    #[test]
    fn test_protocol_display_https() {
        assert_eq!(format!("{}", Protocol::Https), "https");
    }

    // Proptest for protocol parsing
    proptest! {
        #[test]
        fn test_protocol_from_str_never_panics(input in "[a-zA-Z0-9]*") {
            // Property: converting any string to Protocol should never panic
            let _ = Protocol::from(input.as_str());
        }

        #[test]
        fn test_protocol_stdio_case_insensitive(
            variant in "(stdio|STDIO|Stdio|StDiO)"
        ) {
            let protocol = Protocol::from(variant.as_str());
            prop_assert!(matches!(protocol, Protocol::Stdio));
        }

        #[test]
        fn test_protocol_socket_case_insensitive(
            variant in "(socket|SOCKET|Socket|SoCkEt)"
        ) {
            let protocol = Protocol::from(variant.as_str());
            prop_assert!(matches!(protocol, Protocol::Socket));
        }

        #[test]
        fn test_protocol_http_case_insensitive(
            variant in "(http|HTTP|Http|HtTp)"
        ) {
            let protocol = Protocol::from(variant.as_str());
            prop_assert!(matches!(protocol, Protocol::Http));
        }

        #[test]
        fn test_protocol_https_case_insensitive(
            variant in "(https|HTTPS|Https|HtTpS)"
        ) {
            let protocol = Protocol::from(variant.as_str());
            prop_assert!(matches!(protocol, Protocol::Https));
        }
    }
}
