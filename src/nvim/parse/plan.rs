use agent_client_protocol::Plan;
use nvim_oxi::Dictionary;

pub fn plan_event(plan: Plan) -> Dictionary {
    let mut data: nvim_oxi::Dictionary = nvim_oxi::Dictionary::new();
    let entries = plan.entries.into_iter().map(|entry| {
        let mut dict = nvim_oxi::Dictionary::new();
        dict.insert("content", entry.content.to_string());
        dict.insert("priority", format!("{:?}", entry.priority));
        // dict.insert("meta", entry.meta);
        dict
    });

    data.insert("entries", nvim_oxi::Array::from_iter(entries));

    if let Some(meta) = plan.meta {
        data.insert("meta", format!("{:?}", meta));
    }

    data
}
