use agent_client_protocol::schema::v1::{
    NewSessionResponse, SessionConfigKind, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOptions,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct HermesOption {
    pub value: String,
    pub name: String,
    pub description: Option<String>,
    pub group: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Selection {
    pub options: Vec<HermesOption>,
    pub current: HermesOption,
    #[serde(skip)]
    legacy: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ModelConfigOption {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub selection: Selection,
}

#[derive(Debug, Default, Clone)]
pub struct SessionDetails {
    pub modes: Option<Selection>,
    pub models: Option<Selection>,
    pub thought_levels: Option<Selection>,
    pub model_configs: Vec<ModelConfigOption>,
    pub config_options: Vec<SessionConfigOption>,
}

impl SessionDetails {
    pub fn new(session: &NewSessionResponse) -> Self {
        Self {
            modes: Self::parse_modes(session),
            models: Self::parse_models(session),
            thought_levels: Self::parse_thought_levels(session),
            model_configs: Self::parse_model_configs(session),
            config_options: session.config_options.clone().unwrap_or_default(),
        }
    }

    pub fn mode_is_legacy(&self) -> Option<bool> {
        self.modes.as_ref().map(|mode| mode.legacy)
    }

    pub fn mode_options(&self) -> Option<&Vec<HermesOption>> {
        self.modes.as_ref().map(|mode| &mode.options)
    }

    pub fn current_mode(&self) -> Option<&HermesOption> {
        self.modes.as_ref().map(|mode| &mode.current)
    }

    pub fn set_current_mode(&mut self, new_current: HermesOption) {
        if let Some(modes) = &mut self.modes {
            modes.current = new_current;
        }
    }

    pub fn get_mode(&self, value: &str) -> Option<&HermesOption> {
        self.modes
            .as_ref()
            .and_then(|mode| mode.options.iter().find(|option| option.value == value))
    }

    pub fn model_is_legacy(&self) -> Option<bool> {
        self.models.as_ref().map(|model| model.legacy)
    }

    pub fn model_options(&self) -> Option<&Vec<HermesOption>> {
        self.models.as_ref().map(|model| &model.options)
    }

    pub fn current_model(&self) -> Option<&HermesOption> {
        self.models.as_ref().map(|model| &model.current)
    }

    pub fn set_current_model(&mut self, new_current: HermesOption) {
        if let Some(models) = &mut self.models {
            models.current = new_current;
        }
    }

    pub fn get_model(&self, value: &str) -> Option<&HermesOption> {
        self.models
            .as_ref()
            .and_then(|model| model.options.iter().find(|option| option.value == value))
    }

    pub fn set_current_thought_level(&mut self, new_current: HermesOption) {
        if let Some(thought_levels) = &mut self.thought_levels {
            thought_levels.current = new_current;
        }
    }

    pub fn get_thought_level(&self, value: &str) -> Option<&HermesOption> {
        self.thought_levels
            .as_ref()
            .and_then(|tl| tl.options.iter().find(|option| option.value == value))
    }

    pub fn thought_level_options(&self) -> Option<&Vec<HermesOption>> {
        self.thought_levels.as_ref().map(|model| &model.options)
    }

    pub fn current_thought_level(&self) -> Option<&HermesOption> {
        self.thought_levels.as_ref().map(|model| &model.current)
    }

    pub fn model_config_options(&self) -> &[ModelConfigOption] {
        &self.model_configs
    }

    pub fn update_model_config(&mut self, id: &str, new_current: &str) {
        if let Some(mc) = self.model_configs.iter_mut().find(|mc| mc.id == id) {
            if let Some(new_option) = mc
                .selection
                .options
                .iter()
                .find(|o| o.value == new_current)
                .cloned()
            {
                mc.selection.current = new_option;
            }
        }
    }

    pub fn merge_or_replace_model_configs(&mut self, configs: Vec<ModelConfigOption>) {
        for config in configs {
            if let Some(existing) = self.model_configs.iter_mut().find(|mc| mc.id == config.id) {
                *existing = config;
            } else {
                self.model_configs.push(config);
            }
        }
    }

    pub fn get_model_config(&self, id: &str) -> Option<&ModelConfigOption> {
        self.model_configs.iter().find(|mc| mc.id == id)
    }

    /// Every config option the agent has reported, stored verbatim.
    pub fn all_config_options(&self) -> &[SessionConfigOption] {
        &self.config_options
    }

    /// Replace the stored config options with a fresh authoritative snapshot.
    ///
    /// ACP sends the complete option set on session creation, on every
    /// `session/set_config_option` response, and on `config_option_update`
    /// notifications, so a wholesale replacement is always correct.
    pub fn replace_config_options(&mut self, options: Vec<SessionConfigOption>) {
        self.config_options = options;
    }

    fn parse_option_selection(opt: &SessionConfigOption) -> Option<(Vec<HermesOption>, String)> {
        match &opt.kind {
            SessionConfigKind::Select(select) => {
                let current_value = select.current_value.to_string();
                let options = match &select.options {
                    SessionConfigSelectOptions::Grouped(groups) => groups
                        .iter()
                        .flat_map(|group| {
                            group.options.iter().map(|o| HermesOption {
                                value: o.value.to_string(),
                                name: o.name.to_string(),
                                description: o.description.clone(),
                                group: Some(group.name.to_string()),
                            })
                        })
                        .collect(),
                    SessionConfigSelectOptions::Ungrouped(ungrouped) => ungrouped
                        .iter()
                        .map(|o| HermesOption {
                            value: o.value.to_string(),
                            name: o.name.to_string(),
                            description: o.description.clone(),
                            group: None,
                        })
                        .collect(),
                    _ => return None,
                };
                Some((options, current_value))
            }
            _ => None,
        }
    }

    fn parse_options(
        session: &NewSessionResponse,
        category: SessionConfigOptionCategory,
    ) -> Option<Selection> {
        let mut current_option = String::new();
        let options: Vec<HermesOption> = session
            .config_options
            .as_ref()
            .map(|options| {
                options
                    .iter()
                    .filter_map(|opt| {
                        if opt.category.as_ref() == Some(&category) {
                            Self::parse_option_selection(opt).map(|(opts, cur)| {
                                current_option = cur;
                                opts
                            })
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect::<Vec<HermesOption>>()
            })
            .unwrap_or_default();

        if !options.is_empty() {
            let current = options
                .iter()
                .find(|option| option.value == current_option)
                .cloned()
                .unwrap_or_else(|| {
                    tracing::warn!(
                        "Current value '{}' not found in options for category {:?}, defaulting to first option",
                        current_option,
                        category
                    );
                    options[0].clone()
                });

            Some(Selection {
                current,
                options,
                legacy: false,
            })
        } else {
            None
        }
    }

    fn parse_thought_levels(session: &NewSessionResponse) -> Option<Selection> {
        Self::parse_options(session, SessionConfigOptionCategory::ThoughtLevel)
    }

    fn parse_models(session: &NewSessionResponse) -> Option<Selection> {
        Self::parse_options(session, SessionConfigOptionCategory::Model)
    }

    fn parse_model_configs(session: &NewSessionResponse) -> Vec<ModelConfigOption> {
        session
            .config_options
            .as_ref()
            .map(|options| Self::parse_model_configs_from_options(options))
            .unwrap_or_default()
    }

    pub fn parse_model_configs_from_options(
        config_options: &[SessionConfigOption],
    ) -> Vec<ModelConfigOption> {
        config_options
            .iter()
            .filter_map(|opt| {
                if opt.category.as_ref() != Some(&SessionConfigOptionCategory::ModelConfig) {
                    return None;
                }

                let (selection_options, current_value) =
                    Self::parse_option_selection(opt)?;

                if selection_options.is_empty() {
                    return None;
                }

                let current = selection_options
                    .iter()
                    .find(|o| o.value == current_value)
                    .cloned()
                    .unwrap_or_else(|| {
                        tracing::warn!(
                            "Current value '{}' not found in model config '{}', defaulting to first option",
                            current_value,
                            opt.id
                        );
                        selection_options[0].clone()
                    });

                Some(ModelConfigOption {
                    id: opt.id.to_string(),
                    name: opt.name.to_string(),
                    description: opt.description.clone(),
                    selection: Selection {
                        current,
                        options: selection_options,
                        legacy: false,
                    },
                })
            })
            .collect()
    }

    fn parse_modes(session: &NewSessionResponse) -> Option<Selection> {
        Self::parse_options(session, SessionConfigOptionCategory::Mode).or_else(|| {
            let mut current = HermesOption::default();
            session.modes.as_ref().map(|modes| Selection {
                options: modes
                    .available_modes
                    .iter()
                    .map(|mode| {
                        let option = HermesOption {
                            value: mode.id.to_string(),
                            name: mode.name.to_string(),
                            description: mode.description.clone(),
                            group: None,
                        };
                        if option.value == modes.current_mode_id.to_string() {
                            current = option.clone();
                        }
                        option
                    })
                    .collect(),
                current,
                legacy: true,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::{
        NewSessionResponse, SessionConfigOption, SessionConfigOptionCategory,
        SessionConfigSelectGroup, SessionConfigSelectOption, SessionMode, SessionModeState,
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_modes_new_config_path_returns_non_legacy() {
        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![
                SessionConfigSelectOption::new("chat", "Chat"),
                SessionConfigSelectOption::new("code", "Code"),
            ],
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        assert_eq!(details.mode_is_legacy(), Some(false));
    }

    #[test]
    fn parse_modes_new_config_path_stores_options() {
        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![
                SessionConfigSelectOption::new("chat", "Chat"),
                SessionConfigSelectOption::new("code", "Code"),
            ],
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].value, "chat");
        assert_eq!(options[0].name, "Chat");
        assert_eq!(options[0].group, None);
        assert_eq!(options[1].value, "code");
        assert_eq!(options[1].name, "Code");
        assert_eq!(options[1].group, None);
    }

    #[test]
    fn parse_modes_new_config_path_stores_current() {
        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        assert_eq!(
            details.current_mode().map(|m| m.value.as_str()),
            Some("chat")
        );
    }

    #[test]
    fn parse_modes_legacy_fallback_returns_legacy() {
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session").modes(modes);

        let details = SessionDetails::new(&session);
        assert_eq!(details.mode_is_legacy(), Some(true));
    }

    #[test]
    fn parse_modes_legacy_stores_options() {
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session").modes(modes);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].value, "chat");
        assert_eq!(options[0].name, "Chat");
        assert_eq!(options[0].group, None);
    }

    #[test]
    fn parse_modes_neither_present_returns_none() {
        let session = NewSessionResponse::new("test-session");
        let details = SessionDetails::new(&session);
        assert_eq!(details.mode_is_legacy(), None);
        assert_eq!(details.mode_options(), None);
        assert_eq!(details.current_mode(), None);
    }

    #[test]
    fn parse_modes_new_takes_precedence_over_legacy() {
        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        )
        .category(SessionConfigOptionCategory::Mode);

        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session")
            .config_options(vec![option])
            .modes(modes);

        let details = SessionDetails::new(&session);
        assert_eq!(details.mode_is_legacy(), Some(false));
    }

    #[test]
    fn parse_modes_wrong_category_falls_back_to_legacy() {
        let option = SessionConfigOption::select(
            "model",
            "Model",
            "gpt4",
            vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
        )
        .category(SessionConfigOptionCategory::Model);

        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session")
            .config_options(vec![option])
            .modes(modes);

        let details = SessionDetails::new(&session);
        assert_eq!(details.mode_is_legacy(), Some(true));
    }

    #[test]
    fn parse_modes_grouped_flattens_all_groups() {
        let group1 = SessionConfigSelectGroup::new(
            "group1",
            "Group One",
            vec![
                SessionConfigSelectOption::new("chat", "Chat"),
                SessionConfigSelectOption::new("code", "Code"),
            ],
        );
        let group2 = SessionConfigSelectGroup::new(
            "group2",
            "Group Two",
            vec![SessionConfigSelectOption::new("agent", "Agent")],
        );
        let option = SessionConfigOption::new(
            "mode",
            "Mode",
            agent_client_protocol::schema::v1::SessionConfigKind::Select(
                agent_client_protocol::schema::v1::SessionConfigSelect::new(
                    "chat",
                    vec![group1, group2],
                ),
            ),
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options.len(), 3);
    }

    #[test]
    fn parse_modes_grouped_includes_group_name() {
        let group = SessionConfigSelectGroup::new(
            "my-group",
            "My Group",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        );
        let option = SessionConfigOption::new(
            "mode",
            "Mode",
            agent_client_protocol::schema::v1::SessionConfigKind::Select(
                agent_client_protocol::schema::v1::SessionConfigSelect::new("chat", vec![group]),
            ),
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].value, "chat");
        assert_eq!(options[0].name, "Chat");
        assert_eq!(options[0].group, Some("My Group".to_string()));
    }

    #[test]
    fn parse_modes_grouped_current_value_preserved() {
        let group = SessionConfigSelectGroup::new(
            "g",
            "G",
            vec![SessionConfigSelectOption::new("code", "Code")],
        );
        let option = SessionConfigOption::new(
            "mode",
            "Mode",
            agent_client_protocol::schema::v1::SessionConfigKind::Select(
                agent_client_protocol::schema::v1::SessionConfigSelect::new("code", vec![group]),
            ),
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        assert_eq!(
            details.current_mode().map(|m| m.value.as_str()),
            Some("code")
        );
        assert_eq!(details.mode_is_legacy(), Some(false));
    }

    #[test]
    fn parse_modes_ungrouped_has_no_group() {
        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options[0].group, None);
    }

    #[test]
    fn parse_modes_legacy_has_no_group() {
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session").modes(modes);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options[0].group, None);
    }

    #[test]
    fn parse_modes_grouped_description_preserved() {
        let group = SessionConfigSelectGroup::new(
            "g",
            "G",
            vec![SessionConfigSelectOption::new("chat", "Chat").description("Chat mode")],
        );
        let option = SessionConfigOption::new(
            "mode",
            "Mode",
            agent_client_protocol::schema::v1::SessionConfigKind::Select(
                agent_client_protocol::schema::v1::SessionConfigSelect::new("chat", vec![group]),
            ),
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options[0].description, Some("Chat mode".to_string()));
    }

    #[test]
    fn parse_models_new_config_path_returns_non_legacy() {
        let option = SessionConfigOption::select(
            "model",
            "Model",
            "gpt4",
            vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
        )
        .category(SessionConfigOptionCategory::Model);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        assert_eq!(details.model_is_legacy(), Some(false));
    }

    #[test]
    fn parse_models_neither_present_returns_none() {
        let session = NewSessionResponse::new("test-session");
        let details = SessionDetails::new(&session);
        assert!(details.model_is_legacy().is_none());
    }

    // Model config tests

    fn model_config_option(id: &str, name: &str, current: &str) -> SessionConfigOption {
        SessionConfigOption::select(
            id.to_string(),
            name.to_string(),
            current.to_string(),
            vec![SessionConfigSelectOption::new(
                current.to_string(),
                name.to_string(),
            )],
        )
        .category(SessionConfigOptionCategory::ModelConfig)
    }

    #[test]
    fn parse_model_configs_from_options_filters_by_category() {
        let model_conf = model_config_option("mc-1", "MC One", "val1");
        let model_option = SessionConfigOption::select(
            "model",
            "Model",
            "gpt4",
            vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
        )
        .category(SessionConfigOptionCategory::Model);

        let result = SessionDetails::parse_model_configs_from_options(&[model_conf, model_option]);

        assert_eq!(
            result,
            vec![ModelConfigOption {
                id: "mc-1".to_string(),
                name: "MC One".to_string(),
                description: None,
                selection: Selection {
                    options: vec![HermesOption {
                        value: "val1".to_string(),
                        name: "MC One".to_string(),
                        description: None,
                        group: None,
                    }],
                    current: HermesOption {
                        value: "val1".to_string(),
                        name: "MC One".to_string(),
                        description: None,
                        group: None,
                    },
                    legacy: false,
                },
            }]
        );
    }

    #[test]
    fn parse_model_configs_from_options_empty_options() {
        let result = SessionDetails::parse_model_configs_from_options(&[]);

        assert!(result.is_empty());
    }

    #[test]
    fn parse_model_configs_from_options_no_model_config_category() {
        let option = SessionConfigOption::select(
            "model",
            "Model",
            "gpt4",
            vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
        )
        .category(SessionConfigOptionCategory::Model);

        let result = SessionDetails::parse_model_configs_from_options(&[option]);

        assert!(result.is_empty());
    }

    #[test]
    fn parse_model_configs_from_options_parses_ungrouped_options() {
        let option = SessionConfigOption::select(
            "mc-1",
            "MC One",
            "val1",
            vec![SessionConfigSelectOption::new("val1", "Value 1")],
        )
        .category(SessionConfigOptionCategory::ModelConfig);

        let result = SessionDetails::parse_model_configs_from_options(&[option]);

        assert_eq!(
            result,
            vec![ModelConfigOption {
                id: "mc-1".to_string(),
                name: "MC One".to_string(),
                description: None,
                selection: Selection {
                    options: vec![HermesOption {
                        value: "val1".to_string(),
                        name: "Value 1".to_string(),
                        description: None,
                        group: None,
                    }],
                    current: HermesOption {
                        value: "val1".to_string(),
                        name: "Value 1".to_string(),
                        description: None,
                        group: None,
                    },
                    legacy: false,
                },
            }]
        );
    }

    #[test]
    fn parse_model_configs_from_options_parses_grouped_options() {
        let group = SessionConfigSelectGroup::new(
            "g1",
            "Group One",
            vec![SessionConfigSelectOption::new("val1", "Value 1")],
        );
        let option = SessionConfigOption::new(
            "mc-1",
            "MC One",
            agent_client_protocol::schema::v1::SessionConfigKind::Select(
                agent_client_protocol::schema::v1::SessionConfigSelect::new("val1", vec![group]),
            ),
        )
        .category(SessionConfigOptionCategory::ModelConfig);

        let result = SessionDetails::parse_model_configs_from_options(&[option]);

        assert_eq!(
            result,
            vec![ModelConfigOption {
                id: "mc-1".to_string(),
                name: "MC One".to_string(),
                description: None,
                selection: Selection {
                    options: vec![HermesOption {
                        value: "val1".to_string(),
                        name: "Value 1".to_string(),
                        description: None,
                        group: Some("Group One".to_string()),
                    }],
                    current: HermesOption {
                        value: "val1".to_string(),
                        name: "Value 1".to_string(),
                        description: None,
                        group: Some("Group One".to_string()),
                    },
                    legacy: false,
                },
            }]
        );
    }

    #[test]
    fn parse_model_configs_from_options_current_value_preserved() {
        let option = SessionConfigOption::select(
            "mc-1",
            "MC One",
            "val2",
            vec![
                SessionConfigSelectOption::new("val1", "Value 1"),
                SessionConfigSelectOption::new("val2", "Value 2"),
            ],
        )
        .category(SessionConfigOptionCategory::ModelConfig);

        let result = SessionDetails::parse_model_configs_from_options(&[option]);

        assert_eq!(result[0].selection.current.value, "val2");
    }

    #[test]
    fn parse_model_configs_from_options_unknown_category_returns_empty() {
        let option = SessionConfigOption::select(
            "mc-1",
            "MC One",
            "val1",
            vec![SessionConfigSelectOption::new("val1", "Value 1")],
        )
        .category(SessionConfigOptionCategory::Model);

        let result = SessionDetails::parse_model_configs_from_options(&[option]);

        assert!(result.is_empty());
    }

    #[test]
    fn session_details_new_populates_model_configs() {
        let option = model_config_option("mc-1", "MC One", "val1");
        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);

        assert_eq!(
            details.model_configs,
            vec![ModelConfigOption {
                id: "mc-1".to_string(),
                name: "MC One".to_string(),
                description: None,
                selection: Selection {
                    options: vec![HermesOption {
                        value: "val1".to_string(),
                        name: "MC One".to_string(),
                        description: None,
                        group: None,
                    }],
                    current: HermesOption {
                        value: "val1".to_string(),
                        name: "MC One".to_string(),
                        description: None,
                        group: None,
                    },
                    legacy: false,
                },
            }]
        );
    }

    #[test]
    fn model_config_options_returns_configs() {
        let option = model_config_option("mc-1", "MC One", "val1");
        let session = NewSessionResponse::new("test-session").config_options(vec![option]);
        let details = SessionDetails::new(&session);

        assert_eq!(
            details.model_config_options(),
            &[ModelConfigOption {
                id: "mc-1".to_string(),
                name: "MC One".to_string(),
                description: None,
                selection: Selection {
                    options: vec![HermesOption {
                        value: "val1".to_string(),
                        name: "MC One".to_string(),
                        description: None,
                        group: None,
                    }],
                    current: HermesOption {
                        value: "val1".to_string(),
                        name: "MC One".to_string(),
                        description: None,
                        group: None,
                    },
                    legacy: false,
                },
            }]
        );
    }

    #[test]
    fn model_config_options_returns_empty_when_none() {
        let session = NewSessionResponse::new("test-session");
        let details = SessionDetails::new(&session);

        let configs = details.model_config_options();

        assert!(configs.is_empty());
    }

    #[test]
    fn get_model_config_returns_some_for_existing_id() {
        let option = model_config_option("mc-1", "MC One", "val1");
        let session = NewSessionResponse::new("test-session").config_options(vec![option]);
        let details = SessionDetails::new(&session);

        let config = details.get_model_config("mc-1");

        assert_eq!(config.map(|c| c.id.as_str()), Some("mc-1"));
    }

    #[test]
    fn get_model_config_returns_none_for_missing_id() {
        let option = model_config_option("mc-1", "MC One", "val1");
        let session = NewSessionResponse::new("test-session").config_options(vec![option]);
        let details = SessionDetails::new(&session);

        let config = details.get_model_config("nonexistent");

        assert!(config.is_none());
    }

    #[test]
    fn update_model_config_updates_current_value() {
        let option = SessionConfigOption::select(
            "mc-1",
            "MC One",
            "val1",
            vec![
                SessionConfigSelectOption::new("val1", "Value 1"),
                SessionConfigSelectOption::new("val2", "Value 2"),
            ],
        )
        .category(SessionConfigOptionCategory::ModelConfig);
        let session = NewSessionResponse::new("test-session").config_options(vec![option]);
        let mut details = SessionDetails::new(&session);

        details.update_model_config("mc-1", "val2");

        assert_eq!(details.model_configs[0].selection.current.value, "val2");
    }

    #[test]
    fn update_model_config_invalid_id_does_nothing() {
        let option = model_config_option("mc-1", "MC One", "val1");
        let session = NewSessionResponse::new("test-session").config_options(vec![option]);
        let mut details = SessionDetails::new(&session);

        details.update_model_config("nonexistent", "val2");

        assert_eq!(details.model_configs[0].selection.current.value, "val1");
    }

    #[test]
    fn update_model_config_invalid_value_does_nothing() {
        let option = model_config_option("mc-1", "MC One", "val1");
        let session = NewSessionResponse::new("test-session").config_options(vec![option]);
        let mut details = SessionDetails::new(&session);

        details.update_model_config("mc-1", "nonexistent");

        assert_eq!(details.model_configs[0].selection.current.value, "val1");
    }

    #[test]
    fn merge_or_replace_replaces_existing_by_id() {
        let option = model_config_option("mc-1", "MC One", "val1");
        let session = NewSessionResponse::new("test-session").config_options(vec![option]);
        let mut details = SessionDetails::new(&session);

        let replacement = ModelConfigOption {
            id: "mc-1".to_string(),
            name: "MC One Updated".to_string(),
            description: None,
            selection: Selection {
                options: vec![HermesOption {
                    value: "val2".to_string(),
                    name: "Value 2".to_string(),
                    description: None,
                    group: None,
                }],
                current: HermesOption {
                    value: "val2".to_string(),
                    name: "Value 2".to_string(),
                    description: None,
                    group: None,
                },
                legacy: false,
            },
        };

        details.merge_or_replace_model_configs(vec![replacement]);

        assert_eq!(details.model_configs.len(), 1);
        assert_eq!(details.model_configs[0].name, "MC One Updated");
        assert_eq!(details.model_configs[0].selection.current.value, "val2");
    }

    #[test]
    fn merge_or_replace_preserves_unrelated_configs() {
        let opt1 = model_config_option("mc-1", "MC One", "val1");
        let opt2 = model_config_option("mc-2", "MC Two", "val1");
        let session = NewSessionResponse::new("test-session").config_options(vec![opt1, opt2]);
        let mut details = SessionDetails::new(&session);

        let replacement = ModelConfigOption {
            id: "mc-1".to_string(),
            name: "MC One Updated".to_string(),
            description: None,
            selection: Selection {
                options: vec![HermesOption {
                    value: "val2".to_string(),
                    name: "Value 2".to_string(),
                    description: None,
                    group: None,
                }],
                current: HermesOption {
                    value: "val2".to_string(),
                    name: "Value 2".to_string(),
                    description: None,
                    group: None,
                },
                legacy: false,
            },
        };

        details.merge_or_replace_model_configs(vec![replacement]);

        assert_eq!(details.model_configs.len(), 2);
        assert_eq!(details.model_configs[0].name, "MC One Updated");
        assert_eq!(details.model_configs[1].name, "MC Two");
    }

    #[test]
    fn merge_or_replace_appends_new_config() {
        let option = model_config_option("mc-1", "MC One", "val1");
        let session = NewSessionResponse::new("test-session").config_options(vec![option]);
        let mut details = SessionDetails::new(&session);

        let new_config = ModelConfigOption {
            id: "mc-2".to_string(),
            name: "MC Two".to_string(),
            description: None,
            selection: Selection {
                options: vec![HermesOption {
                    value: "val1".to_string(),
                    name: "Value 1".to_string(),
                    description: None,
                    group: None,
                }],
                current: HermesOption {
                    value: "val1".to_string(),
                    name: "Value 1".to_string(),
                    description: None,
                    group: None,
                },
                legacy: false,
            },
        };

        details.merge_or_replace_model_configs(vec![new_config]);

        assert_eq!(details.model_configs.len(), 2);
        assert_eq!(details.model_configs[0].id, "mc-1");
        assert_eq!(details.model_configs[1].id, "mc-2");
    }

    #[test]
    fn merge_or_replace_empty_existing_list() {
        let session = NewSessionResponse::new("test-session");
        let mut details = SessionDetails::new(&session);

        let new_config = ModelConfigOption {
            id: "mc-1".to_string(),
            name: "MC One".to_string(),
            description: None,
            selection: Selection {
                options: vec![HermesOption {
                    value: "val1".to_string(),
                    name: "Value 1".to_string(),
                    description: None,
                    group: None,
                }],
                current: HermesOption {
                    value: "val1".to_string(),
                    name: "Value 1".to_string(),
                    description: None,
                    group: None,
                },
                legacy: false,
            },
        };

        details.merge_or_replace_model_configs(vec![new_config]);

        assert_eq!(details.model_configs.len(), 1);
        assert_eq!(details.model_configs[0].id, "mc-1");
    }
}
