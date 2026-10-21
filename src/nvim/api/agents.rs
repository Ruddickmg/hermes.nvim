use nvim_oxi::conversion::FromObject;
use serde::Serialize;
use std::collections::HashMap;

use crate::{
    acp::{
        self,
        registry::{distribution::Distribution, entry::AgentEntry},
    },
    api::Api,
    nvim::autocommands::Commands,
    utilities::icon_renderer::{IconRenderer, PixelIcon},
};

const DEFAULT_REGISTRY_URL: &str =
    "https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json";

#[derive(Debug, Clone, Default)]
pub struct AgentsConfig {
    pub update: Option<bool>,
    pub url: Option<String>,
}

impl nvim_oxi::conversion::FromObject for AgentsConfig {
    fn from_object(obj: nvim_oxi::Object) -> Result<Self, nvim_oxi::conversion::Error> {
        if obj.is_nil() {
            return Ok(Self::default());
        }

        let dict: nvim_oxi::Dictionary = crate::nvim::configuration::dict_from_object(obj)?;

        let update: Option<bool> = dict
            .get("update")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;

        let url: Option<String> = dict.get("url").and_then(|o| {
            TryInto::<nvim_oxi::String>::try_into(o.clone())
                .ok()
                .map(|s: nvim_oxi::String| s.to_string())
        });

        Ok(Self { update, url })
    }
}

impl nvim_oxi::lua::Poppable for AgentsConfig {
    unsafe fn pop(lua_state: *mut nvim_oxi::lua::ffi::State) -> Result<Self, nvim_oxi::lua::Error> {
        let obj = unsafe { nvim_oxi::Object::pop(lua_state)? };
        Ok(Self::from_object(obj)
            .inspect_err(|e| tracing::error!("{:?}", e))
            .unwrap_or_default())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentListEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_pixel_data: Option<PixelIcon>,
    pub distributions: Vec<Distribution>,
}

#[derive(Debug, Serialize)]
struct AgentListPayload {
    agents: Vec<AgentListEntry>,
}

impl From<AgentEntry> for AgentListEntry {
    fn from(entry: AgentEntry) -> Self {
        Self {
            distributions: entry.distributions(),
            id: entry.id,
            name: entry.name,
            version: entry.version,
            license: entry.license,
            description: entry.description,
            website: entry.website,
            repository: entry.repository,
            icon: entry.icon,
            icon_pixel_data: None,
        }
    }
}

impl AgentListEntry {
    /// Build a list entry for a user-configured agent that has no registry match.
    pub fn from_custom(
        id: &str,
        config: &crate::nvim::configuration::AgentConnectionConfig,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: config.connection.name(),
            version: String::new(),
            license: None,
            description: config.description.clone().unwrap_or_default(),
            website: None,
            repository: None,
            icon: config.icon.clone(),
            icon_pixel_data: None,
            distributions: Vec::new(),
        }
    }
}

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn agents(&self, maybe_config: Option<AgentsConfig>) -> crate::acp::Result<()> {
        let config = maybe_config.unwrap_or_default();
        let url = config.url.as_deref().unwrap_or(DEFAULT_REGISTRY_URL);
        let update = config.update.unwrap_or(false);
        let state = self.state.lock().await;
        let registry = state.registry.clone();
        let config_distributions = state.config.distributions.clone();
        let mut custom_agents = state.config.agents.clone();
        let icon_cache_dir = state.config.project_root.join(".hermes").join("icons");
        drop(state);

        if update {
            let current = registry
                .as_ref()
                .ok_or_else(|| acp::error::Error::Network("No registry loaded".to_string()))?;
            match current.fetch(url).await {
                Ok(reg) => {
                    let mut state = self.state.lock().await;
                    state.registry = Some(reg.clone());
                    drop(state);
                }
                Err(e) => {
                    if registry.is_none() {
                        return Err(acp::error::Error::Network(e.to_string()));
                    }
                }
            }
        }

        let registered_agents: Vec<AgentListEntry> = registry
            .as_ref()
            .map(|r| {
                r.data
                    .agents
                    .values()
                    .cloned()
                    .map(|agent| AgentListEntry {
                        distributions: agent
                            .distributions()
                            .into_iter()
                            .filter(|distribution| distribution.is_enabled(&config_distributions))
                            .collect(),
                        ..agent.into()
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut agents = registered_agents
            .into_iter()
            .map(|agent| {
                custom_agents
                    .remove(&agent.id)
                    .map(|entry| AgentListEntry::from_custom(&agent.id, &entry))
                    .unwrap_or(agent)
            })
            .collect::<Vec<AgentListEntry>>();
        let additional_agents: Vec<AgentListEntry> = custom_agents.into();

        agents.extend(additional_agents);
        agents.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        let renderer = IconRenderer::new(icon_cache_dir);
        let render_tasks: Vec<_> = agents
            .iter()
            .filter_map(|entry| {
                let url = entry.icon.as_ref()?;
                let renderer = renderer.clone();
                let url = url.clone();
                Some(async move {
                    (
                        url.clone(),
                        renderer.get_or_render(&url).await.ok().flatten(),
                    )
                })
            })
            .collect();

        let results: HashMap<String, Option<PixelIcon>> = futures::future::join_all(render_tasks)
            .await
            .into_iter()
            .collect();

        for entry in &mut agents {
            entry.icon_pixel_data = match &entry.icon {
                Some(url) => results.get(url).cloned().flatten(),
                None => Some(IconRenderer::default_icon()),
            };
        }

        let _ = self
            .response_handler
            .execute_autocommand(Commands::AgentList, AgentListPayload { agents })
            .await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::connection::Assistant;
    use crate::acp::registry::{DistributionCommand, PackageDistribution};
    use crate::nvim::configuration::{BinaryConfig, DistributionsConfig};
    use nvim_oxi::{Dictionary, Object};
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    fn filtered_distributions(
        entry: &AgentEntry,
        config: &DistributionsConfig,
    ) -> Vec<Distribution> {
        entry
            .distributions()
            .into_iter()
            .filter(|d| d.is_enabled(config))
            .collect()
    }

    fn make_multi_distribution_entry() -> AgentEntry {
        AgentEntry {
            id: "multi-agent".to_string(),
            name: "Multi Agent".to_string(),
            version: "1.0.0".to_string(),
            description: "An agent with multiple distributions".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: None,
            icon: None,
            distribution: HashMap::from([
                (
                    Distribution::Npx,
                    DistributionCommand::Package(PackageDistribution {
                        package: "npx-pkg".to_string(),
                        args: None,
                        env: None,
                    }),
                ),
                (
                    Distribution::Uvx,
                    DistributionCommand::Package(PackageDistribution {
                        package: "uvx-pkg".to_string(),
                        args: None,
                        env: None,
                    }),
                ),
                (
                    Distribution::Binary,
                    DistributionCommand::BinaryTargets(HashMap::new()),
                ),
            ]),
        }
    }

    #[test]
    fn test_agents_config_from_object_nil() {
        let config = AgentsConfig::from_object(Object::nil()).unwrap();
        assert!(
            config.update.is_none() && config.url.is_none(),
            "Nil should produce default config"
        );
    }

    #[test]
    fn test_agents_config_from_object_empty_dict() {
        let config = AgentsConfig::from_object(Object::from(Dictionary::new())).unwrap();
        assert!(
            config.update.is_none() && config.url.is_none(),
            "Empty dict should produce default config"
        );
    }

    #[test]
    fn test_agents_config_from_object_update_true() {
        let mut dict = Dictionary::new();
        dict.insert("update", true);
        let config = AgentsConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(config.update, Some(true));
    }

    #[test]
    fn test_agents_config_from_object_update_false() {
        let mut dict = Dictionary::new();
        dict.insert("update", false);
        let config = AgentsConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(config.update, Some(false));
    }

    #[test]
    fn test_agents_config_from_object_url() {
        let mut dict = Dictionary::new();
        dict.insert("url", "https://example.com/registry.json");
        let config = AgentsConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(
            config.url,
            Some("https://example.com/registry.json".to_string())
        );
    }

    #[test]
    fn test_agents_config_from_object_update_and_url_parses_update() {
        let mut dict = Dictionary::new();
        dict.insert("update", true);
        dict.insert("url", "https://example.com/registry.json");
        let config = AgentsConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(config.update, Some(true));
    }

    #[test]
    fn test_agents_config_from_object_update_and_url_parses_url() {
        let mut dict = Dictionary::new();
        dict.insert("update", true);
        dict.insert("url", "https://example.com/registry.json");
        let config = AgentsConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(
            config.url,
            Some("https://example.com/registry.json".to_string())
        );
    }

    #[test]
    fn test_agents_config_from_object_unknown_fields_ignored() {
        let mut dict = Dictionary::new();
        dict.insert("unknown", "value");
        dict.insert("update", true);
        let config = AgentsConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(config.update, Some(true));
    }

    #[test]
    fn test_agents_config_from_object_invalid_type_errors() {
        let arr = nvim_oxi::Array::from_iter(vec![Object::from("test")]);
        let result = AgentsConfig::from_object(Object::from(arr));
        assert!(result.is_err(), "Array should not be a valid AgentsConfig");
    }

    fn make_binary_distribution_entry() -> AgentEntry {
        AgentEntry {
            id: "test-agent".to_string(),
            name: "Test Agent".to_string(),
            version: "1.0.0".to_string(),
            description: "A test agent".to_string(),
            repository: Some("https://github.com/test/repo".to_string()),
            website: Some("https://test.dev".to_string()),
            authors: Some(vec!["test".to_string()]),
            license: Some("MIT".to_string()),
            icon: Some("https://test.dev/icon.png".to_string()),
            distribution: HashMap::from([(
                Distribution::Binary,
                DistributionCommand::BinaryTargets(HashMap::new()),
            )]),
        }
    }

    #[test]
    fn test_agent_list_entry_from_binary_distribution_preserves_id() {
        let list_entry = AgentListEntry::from(make_binary_distribution_entry());
        assert_eq!(list_entry.id, "test-agent");
    }

    #[test]
    fn test_agent_list_entry_from_binary_distribution_preserves_name() {
        let list_entry = AgentListEntry::from(make_binary_distribution_entry());
        assert_eq!(list_entry.name, "Test Agent");
    }

    #[test]
    fn test_agent_list_entry_from_binary_distribution_preserves_version() {
        let list_entry = AgentListEntry::from(make_binary_distribution_entry());
        assert_eq!(list_entry.version, "1.0.0");
    }

    #[test]
    fn test_agent_list_entry_from_binary_distribution_preserves_description() {
        let list_entry = AgentListEntry::from(make_binary_distribution_entry());
        assert_eq!(list_entry.description, "A test agent");
    }

    #[test]
    fn test_agent_list_entry_from_binary_distribution_preserves_repository() {
        let list_entry = AgentListEntry::from(make_binary_distribution_entry());
        assert_eq!(
            list_entry.repository,
            Some("https://github.com/test/repo".to_string())
        );
    }

    #[test]
    fn test_agent_list_entry_from_binary_distribution_preserves_website() {
        let list_entry = AgentListEntry::from(make_binary_distribution_entry());
        assert_eq!(list_entry.website, Some("https://test.dev".to_string()));
    }

    #[test]
    fn test_agent_list_entry_from_binary_distribution_preserves_license() {
        let list_entry = AgentListEntry::from(make_binary_distribution_entry());
        assert_eq!(list_entry.license, Some("MIT".to_string()));
    }

    #[test]
    fn test_agent_list_entry_from_binary_distribution_preserves_icon() {
        let list_entry = AgentListEntry::from(make_binary_distribution_entry());
        assert_eq!(
            list_entry.icon,
            Some("https://test.dev/icon.png".to_string())
        );
    }

    #[test]
    fn test_agent_list_entry_from_binary_distribution_computes_distributions() {
        let list_entry = AgentListEntry::from(make_binary_distribution_entry());
        assert_eq!(list_entry.distributions, vec![Distribution::Binary]);
    }

    fn make_npx_distribution_entry() -> AgentEntry {
        AgentEntry {
            id: "npx-agent".to_string(),
            name: "NPX Agent".to_string(),
            version: "2.0.0".to_string(),
            description: "An NPX agent".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: None,
            icon: None,
            distribution: HashMap::from([(
                Distribution::Npx,
                DistributionCommand::Package(PackageDistribution {
                    package: "npx-pkg".to_string(),
                    args: None,
                    env: None,
                }),
            )]),
        }
    }

    #[test]
    fn test_agent_list_entry_from_npx_distribution_preserves_id() {
        let list_entry = AgentListEntry::from(make_npx_distribution_entry());
        assert_eq!(list_entry.id, "npx-agent");
    }

    #[test]
    fn test_agent_list_entry_from_npx_distribution_preserves_name() {
        let list_entry = AgentListEntry::from(make_npx_distribution_entry());
        assert_eq!(list_entry.name, "NPX Agent");
    }

    #[test]
    fn test_agent_list_entry_from_npx_distribution_repository_is_none() {
        let list_entry = AgentListEntry::from(make_npx_distribution_entry());
        assert!(list_entry.repository.is_none());
    }

    #[test]
    fn test_agent_list_entry_from_npx_distribution_website_is_none() {
        let list_entry = AgentListEntry::from(make_npx_distribution_entry());
        assert!(list_entry.website.is_none());
    }

    #[test]
    fn test_agent_list_entry_from_npx_distribution_license_is_none() {
        let list_entry = AgentListEntry::from(make_npx_distribution_entry());
        assert!(list_entry.license.is_none());
    }

    #[test]
    fn test_agent_list_entry_from_npx_distribution_icon_is_none() {
        let list_entry = AgentListEntry::from(make_npx_distribution_entry());
        assert!(list_entry.icon.is_none());
    }

    #[test]
    fn test_agent_list_entry_from_npx_distribution_computes_distributions() {
        let list_entry = AgentListEntry::from(make_npx_distribution_entry());
        assert_eq!(list_entry.distributions, vec![Distribution::Npx]);
    }

    fn make_uvx_distribution_entry() -> AgentEntry {
        AgentEntry {
            id: "uvx-agent".to_string(),
            name: "UVX Agent".to_string(),
            version: "3.0.0".to_string(),
            description: "A UVX agent".to_string(),
            repository: Some("https://github.com/uvx/repo".to_string()),
            website: None,
            authors: Some(vec!["author".to_string()]),
            license: Some("Apache-2.0".to_string()),
            icon: None,
            distribution: HashMap::from([(
                Distribution::Uvx,
                DistributionCommand::Package(PackageDistribution {
                    package: "uvx-pkg".to_string(),
                    args: None,
                    env: None,
                }),
            )]),
        }
    }

    #[test]
    fn test_agent_list_entry_from_uvx_distribution_preserves_id() {
        let list_entry = AgentListEntry::from(make_uvx_distribution_entry());
        assert_eq!(list_entry.id, "uvx-agent");
    }

    #[test]
    fn test_agent_list_entry_from_uvx_distribution_computes_distributions() {
        let list_entry = AgentListEntry::from(make_uvx_distribution_entry());
        assert_eq!(list_entry.distributions, vec![Distribution::Uvx]);
    }

    #[test]
    fn test_agent_list_payload_serialization_serializes_id() {
        let entry = AgentEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            description: "desc".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: Some("MIT".to_string()),
            icon: None,
            distribution: HashMap::from([(
                Distribution::Binary,
                DistributionCommand::BinaryTargets(HashMap::new()),
            )]),
        };
        let payload = AgentListPayload {
            agents: vec![AgentListEntry::from(entry)],
        };
        let json = serde_json::to_value(payload).unwrap();
        assert_eq!(json["agents"][0]["id"], "test");
    }

    #[test]
    fn test_agent_list_payload_serialization_serializes_distributions() {
        let entry = AgentEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            description: "desc".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: Some("MIT".to_string()),
            icon: None,
            distribution: HashMap::from([(
                Distribution::Binary,
                DistributionCommand::BinaryTargets(HashMap::new()),
            )]),
        };
        let payload = AgentListPayload {
            agents: vec![AgentListEntry::from(entry)],
        };
        let json = serde_json::to_value(payload).unwrap();
        assert_eq!(json["agents"][0]["distributions"][0], "binary");
    }

    #[test]
    fn test_agent_list_payload_serialization_serializes_license() {
        let entry = AgentEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            description: "desc".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: Some("MIT".to_string()),
            icon: None,
            distribution: HashMap::from([(
                Distribution::Binary,
                DistributionCommand::BinaryTargets(HashMap::new()),
            )]),
        };
        let payload = AgentListPayload {
            agents: vec![AgentListEntry::from(entry)],
        };
        let json = serde_json::to_value(payload).unwrap();
        assert_eq!(json["agents"][0].get("license").unwrap(), "MIT");
    }

    #[test]
    fn test_agent_list_payload_serialization_serializes_description() {
        let entry = AgentEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            description: "desc".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: Some("MIT".to_string()),
            icon: None,
            distribution: HashMap::from([(
                Distribution::Binary,
                DistributionCommand::BinaryTargets(HashMap::new()),
            )]),
        };
        let payload = AgentListPayload {
            agents: vec![AgentListEntry::from(entry)],
        };
        let json = serde_json::to_value(payload).unwrap();
        assert_eq!(json["agents"][0]["description"], "desc");
    }

    #[test]
    fn test_agent_list_payload_serialization_serializes_name() {
        let entry = AgentEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            description: "desc".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: Some("MIT".to_string()),
            icon: None,
            distribution: HashMap::from([(
                Distribution::Binary,
                DistributionCommand::BinaryTargets(HashMap::new()),
            )]),
        };
        let payload = AgentListPayload {
            agents: vec![AgentListEntry::from(entry)],
        };
        let json = serde_json::to_value(payload).unwrap();
        assert_eq!(json["agents"][0]["name"], "Test");
    }

    #[test]
    fn test_agent_list_payload_serialization_serializes_version() {
        let entry = AgentEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            description: "desc".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: Some("MIT".to_string()),
            icon: None,
            distribution: HashMap::from([(
                Distribution::Binary,
                DistributionCommand::BinaryTargets(HashMap::new()),
            )]),
        };
        let payload = AgentListPayload {
            agents: vec![AgentListEntry::from(entry)],
        };
        let json = serde_json::to_value(payload).unwrap();
        assert_eq!(json["agents"][0]["version"], "1.0");
    }

    #[test]
    fn filter_keeps_enabled_npx() {
        let entry = make_npx_distribution_entry();
        let config = DistributionsConfig {
            npx: true,
            ..Default::default()
        };
        let result = filtered_distributions(&entry, &config);
        assert_eq!(result, vec![Distribution::Npx]);
    }

    #[test]
    fn filter_removes_disabled_npx() {
        let entry = make_npx_distribution_entry();
        let config = DistributionsConfig {
            npx: false,
            ..Default::default()
        };
        let result = filtered_distributions(&entry, &config);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_keeps_enabled_uvx() {
        let entry = make_uvx_distribution_entry();
        let config = DistributionsConfig {
            uvx: true,
            ..Default::default()
        };
        let result = filtered_distributions(&entry, &config);
        assert_eq!(result, vec![Distribution::Uvx]);
    }

    #[test]
    fn filter_removes_disabled_uvx() {
        let entry = make_uvx_distribution_entry();
        let config = DistributionsConfig {
            uvx: false,
            ..Default::default()
        };
        let result = filtered_distributions(&entry, &config);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_keeps_enabled_binary() {
        let entry = make_binary_distribution_entry();
        let config = DistributionsConfig {
            binary: BinaryConfig {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = filtered_distributions(&entry, &config);
        assert_eq!(result, vec![Distribution::Binary]);
    }

    #[test]
    fn filter_removes_disabled_binary() {
        let entry = make_binary_distribution_entry();
        let config = DistributionsConfig {
            binary: BinaryConfig {
                enabled: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = filtered_distributions(&entry, &config);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_mixed_distributions() {
        let entry = make_multi_distribution_entry();
        let config = DistributionsConfig {
            npx: true,
            uvx: false,
            binary: BinaryConfig {
                enabled: true,
                ..Default::default()
            },
        };
        let result = filtered_distributions(&entry, &config);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&Distribution::Npx));
        assert!(result.contains(&Distribution::Binary));
    }

    #[test]
    fn filter_all_disabled_returns_empty() {
        let entry = make_multi_distribution_entry();
        let config = DistributionsConfig {
            npx: false,
            uvx: false,
            binary: BinaryConfig {
                enabled: false,
                ..Default::default()
            },
        };
        let result = filtered_distributions(&entry, &config);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_removes_invalid() {
        let entry = AgentEntry {
            id: "invalid-dist".to_string(),
            name: "Invalid".to_string(),
            version: "1.0".to_string(),
            description: "".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: None,
            icon: None,
            distribution: HashMap::from([(
                Distribution::Invalid,
                DistributionCommand::Package(PackageDistribution {
                    package: "pkg".to_string(),
                    args: None,
                    env: None,
                }),
            )]),
        };
        let config = DistributionsConfig::default();
        let result = filtered_distributions(&entry, &config);
        assert!(result.is_empty());
    }

    #[test]
    fn agent_list_entry_serialization_includes_icon_pixel_data() {
        let pixel_icon = PixelIcon {
            width: 16,
            height: 16,
            pixels: vec![255u8; 256],
        };
        let entry = AgentListEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            license: Some("MIT".to_string()),
            description: "desc".to_string(),
            website: None,
            repository: None,
            icon: Some("https://example.com/icon.svg".to_string()),
            icon_pixel_data: Some(pixel_icon),
            distributions: vec![Distribution::Binary],
        };
        let json = serde_json::to_value(entry).unwrap();
        let pixel_data = json["icon_pixel_data"].as_object().unwrap();
        assert_eq!(
            (
                pixel_data["width"].as_u64(),
                pixel_data["height"].as_u64(),
                pixel_data["pixels"].as_array().map(|_| true),
            ),
            (Some(16), Some(16), Some(true))
        );
    }

    #[test]
    fn agent_list_entry_serialization_skips_icon_pixel_data_when_none() {
        let entry = AgentListEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            license: None,
            description: "desc".to_string(),
            website: None,
            repository: None,
            icon: None,
            icon_pixel_data: None,
            distributions: vec![Distribution::Npx],
        };
        let json = serde_json::to_value(entry).unwrap();
        assert!(json.get("icon_pixel_data").is_none());
    }

    #[test]
    fn agents_config_from_object_url_as_non_string_errors() {
        let mut dict = Dictionary::new();
        dict.insert("url", 42);
        let config = AgentsConfig::from_object(Object::from(dict)).unwrap();
        assert!(config.url.is_none(), "non-string url should produce None");
    }

    #[test]
    fn agent_list_entry_from_entry_with_all_optionals_set() {
        let entry = AgentEntry {
            id: "full-agent".to_string(),
            name: "Full Agent".to_string(),
            version: "2.0.0".to_string(),
            description: "full description".to_string(),
            repository: Some("https://github.com/full/repo".to_string()),
            website: Some("https://full.dev".to_string()),
            authors: Some(vec!["author1".to_string(), "author2".to_string()]),
            license: Some("Apache-2.0".to_string()),
            icon: Some("https://full.dev/icon.svg".to_string()),
            distribution: HashMap::from([(
                Distribution::Npx,
                DistributionCommand::Package(PackageDistribution {
                    package: "full-pkg".to_string(),
                    args: None,
                    env: None,
                }),
            )]),
        };
        let list_entry = AgentListEntry::from(entry);
        assert_eq!(list_entry.id, "full-agent");
    }

    #[test]
    fn agent_list_entry_from_entry_preserves_all_optionals_when_none() {
        let entry = AgentEntry {
            id: "minimal-agent".to_string(),
            name: "Minimal".to_string(),
            version: "1.0".to_string(),
            description: "".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: None,
            icon: None,
            distribution: HashMap::new(),
        };
        let list_entry = AgentListEntry::from(entry);
        assert!(list_entry.license.is_none());
    }

    #[test]
    fn agent_list_entry_icon_pixel_data_always_none_on_conversion() {
        let entry = AgentEntry {
            id: "icon-agent".to_string(),
            name: "Icon Agent".to_string(),
            version: "1.0".to_string(),
            description: "has icon".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: None,
            icon: Some("https://example.com/icon.svg".to_string()),
            distribution: HashMap::from([(
                Distribution::Binary,
                DistributionCommand::BinaryTargets(HashMap::new()),
            )]),
        };
        let list_entry = AgentListEntry::from(entry);
        assert!(list_entry.icon_pixel_data.is_none());
    }

    #[test]
    fn agent_list_payload_serialization_skips_license_when_none() {
        let entry = AgentListEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            license: None,
            description: "desc".to_string(),
            website: None,
            repository: None,
            icon: None,
            icon_pixel_data: None,
            distributions: vec![Distribution::Binary],
        };
        let payload = AgentListPayload {
            agents: vec![entry],
        };
        let json = serde_json::to_value(payload).unwrap();
        assert!(json["agents"][0].get("license").is_none());
    }

    #[test]
    fn agent_list_payload_serialization_skips_website_when_none() {
        let entry = AgentListEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            license: None,
            description: "desc".to_string(),
            website: None,
            repository: None,
            icon: None,
            icon_pixel_data: None,
            distributions: vec![Distribution::Binary],
        };
        let payload = AgentListPayload {
            agents: vec![entry],
        };
        let json = serde_json::to_value(payload).unwrap();
        assert!(json["agents"][0].get("website").is_none());
    }

    #[test]
    fn agent_list_entry_from_custom_defaults_name_to_id() {
        let config = crate::nvim::configuration::AgentConnectionConfig {
            description: None,
            icon: None,
            protocol: crate::acp::connection::Protocol::Stdio,
            connection: Assistant::CustomStdio {
                name: String::new(),
                command: "my-agent".to_string(),
                args: Vec::new(),
            },
        };
        let entry = AgentListEntry::from_custom("my-agent", &config);
        assert_eq!(entry.name, "my-agent");
    }

    #[test]
    fn agent_list_entry_from_custom_uses_provided_name() {
        let config = crate::nvim::configuration::AgentConnectionConfig {
            description: None,
            icon: None,
            protocol: crate::acp::connection::Protocol::Stdio,
            connection: Assistant::CustomStdio {
                name: String::new(),
                command: "my-agent".to_string(),
                args: Vec::new(),
            },
        };
        let entry = AgentListEntry::from_custom("my-agent", &config);
        assert_eq!(entry.name, "My Agent");
    }

    #[test]
    fn agent_list_payload_serialization_skips_repository_when_none() {
        let entry = AgentListEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            license: None,
            description: "desc".to_string(),
            website: None,
            repository: None,
            icon: None,
            icon_pixel_data: None,
            distributions: vec![Distribution::Binary],
        };
        let payload = AgentListPayload {
            agents: vec![entry],
        };
        let json = serde_json::to_value(payload).unwrap();
        assert!(json["agents"][0].get("repository").is_none());
    }

    #[test]
    fn agent_list_payload_serialization_includes_website_when_some() {
        let entry = AgentListEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            license: None,
            description: "desc".to_string(),
            website: Some("https://example.com".to_string()),
            repository: None,
            icon: None,
            icon_pixel_data: None,
            distributions: vec![Distribution::Binary],
        };
        let payload = AgentListPayload {
            agents: vec![entry],
        };
        let json = serde_json::to_value(payload).unwrap();
        assert_eq!(json["agents"][0]["website"], "https://example.com");
    }
}
