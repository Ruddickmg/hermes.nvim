pub mod authenticate_response;
pub mod config_option_response;
pub mod initialize_response;
pub mod mode_response;
pub mod modes;
pub mod new_session_response;
pub mod prompt_response;
pub mod session_forked_response;
pub mod session_loaded_response;
pub mod session_model_response;
pub mod session_resumed_response;
pub mod sessions_listed_response;

pub use authenticate_response::*;
pub use config_option_response::*;
pub use initialize_response::*;
pub use mode_response::*;
pub use modes::*;
pub use new_session_response::*;
pub use prompt_response::*;
pub use session_forked_response::*;
pub use session_loaded_response::*;
pub use session_model_response::*;
pub use session_resumed_response::*;
pub use sessions_listed_response::*;

use agent_client_protocol::SessionConfigOption;
use nvim_oxi::Dictionary;

pub fn parse_session_config_option(opt: SessionConfigOption) -> Dictionary {
    let mut dict = nvim_oxi::Dictionary::new();
    dict.insert("id", opt.id.0.as_ref().to_string());
    dict.insert("name", opt.name);
    if let Some(description) = opt.description {
        dict.insert("description", description);
    }
    if let Some(category) = opt.category {
        dict.insert("category", format!("{:#?}", category));
    }
    if let agent_client_protocol::SessionConfigKind::Select(selected) = opt.kind {
        let mut select_dict = nvim_oxi::Dictionary::new();
        select_dict.insert(
            "currentValue",
            selected.current_value.0.as_ref().to_string(),
        );
        let options = match selected.options {
            agent_client_protocol::SessionConfigSelectOptions::Ungrouped(opts) => {
                nvim_oxi::Array::from_iter(opts.into_iter().map(|o| {
                    let mut opt_dict = nvim_oxi::Dictionary::new();
                    opt_dict.insert("value", o.value.0.as_ref().to_string());
                    opt_dict.insert("name", o.name);
                    if let Some(desc) = o.description {
                        opt_dict.insert("description", desc);
                    }
                    opt_dict
                }))
            }
            agent_client_protocol::SessionConfigSelectOptions::Grouped(groups) => {
                nvim_oxi::Array::from_iter(groups.into_iter().map(|g| {
                    let mut group_dict = nvim_oxi::Dictionary::new();
                    group_dict.insert("group", g.group.0.as_ref().to_string());
                    group_dict.insert("name", g.name);
                    let opts_array = nvim_oxi::Array::from_iter(g.options.into_iter().map(|o| {
                        let mut opt_dict = nvim_oxi::Dictionary::new();
                        opt_dict.insert("value", o.value.0.as_ref().to_string());
                        opt_dict.insert("name", o.name);
                        if let Some(desc) = o.description {
                            opt_dict.insert("description", desc);
                        }
                        opt_dict
                    }));
                    group_dict.insert("options", opts_array);
                    group_dict
                }))
            }
            _ => nvim_oxi::Array::new(),
        };
        select_dict.insert("options", options);
        dict.insert("kind", select_dict);
    }
    dict
}
