use crate::nvim::parse::convert_metadata_to_lua_object;
use agent_client_protocol::{ConfigOptionUpdate, SessionConfigKind};
use nvim_oxi::Dictionary;

pub fn config_option_event(update: ConfigOptionUpdate) -> Dictionary {
    let mut data: nvim_oxi::Dictionary = nvim_oxi::Dictionary::new();
    let config_options = update.config_options.into_iter().map(|opt| {
        let mut dict = nvim_oxi::Dictionary::new();
        dict.insert("id", opt.id.to_string());
        dict.insert("name", opt.name);
        if let Some(description) = opt.description {
            dict.insert("description", description);
        }
        if let Some(category) = opt.category {
            dict.insert("category", format!("{:?}", category));
        }
        if let SessionConfigKind::Select(selected) = opt.kind {
            let mut select_dict = nvim_oxi::Dictionary::new();
            select_dict.insert("currentValue", selected.current_value.to_string());
            let options = match selected.options {
                agent_client_protocol::SessionConfigSelectOptions::Ungrouped(opts) => {
                    nvim_oxi::Array::from_iter(opts.into_iter().map(|o| {
                        let mut opt_dict = nvim_oxi::Dictionary::new();
                        opt_dict.insert("value", o.value.to_string());
                        opt_dict.insert("name", o.name);
                        opt_dict.insert("type", "ungrouped");
                        if let Some(desc) = o.description {
                            opt_dict.insert("description", desc);
                        }
                        opt_dict
                    }))
                }
                agent_client_protocol::SessionConfigSelectOptions::Grouped(groups) => {
                    nvim_oxi::Array::from_iter(groups.into_iter().map(|g| {
                        let mut group_dict = nvim_oxi::Dictionary::new();
                        group_dict.insert("type", "grouped");
                        group_dict.insert("group", g.group.to_string());
                        group_dict.insert("name", g.name);
                        group_dict.insert(
                            "options",
                            nvim_oxi::Array::from_iter(g.options.into_iter().map(|o| {
                                let mut opt_dict = nvim_oxi::Dictionary::new();
                                opt_dict.insert("value", o.value.to_string());
                                opt_dict.insert("name", o.name);
                                if let Some(desc) = o.description {
                                    opt_dict.insert("description", desc);
                                }
                                opt_dict
                            })),
                        );
                        group_dict
                    }))
                }
                _ => nvim_oxi::Array::new(),
            };
            select_dict.insert("options", options);
            dict.insert("kind", select_dict);
        }
        dict
    });
    data.insert("options", nvim_oxi::Array::from_iter(config_options));

    if let Some(obj) = convert_metadata_to_lua_object(update.meta) {
        data.insert("meta", obj);
    }

    data
}
