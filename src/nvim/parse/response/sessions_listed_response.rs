use agent_client_protocol::ListSessionsResponse;
use nvim_oxi::Dictionary;

pub fn sessions_listed_response(response: ListSessionsResponse) -> Dictionary {
    let mut data = nvim_oxi::Dictionary::new();

    let sessions_arr = nvim_oxi::Array::from_iter(response.sessions.into_iter().map(|info| {
        let mut info_dict = nvim_oxi::Dictionary::new();
        info_dict.insert("sessionId", info.session_id.0.as_ref().to_string());
        info_dict.insert("cwd", info.cwd.to_string_lossy().as_ref());
        if let Some(title) = info.title {
            info_dict.insert("title", title);
        }
        if let Some(updated_at) = info.updated_at {
            info_dict.insert("updatedAt", updated_at.as_str());
        }
        info_dict
    }));
    data.insert("sessions", sessions_arr);

    if let Some(next_cursor) = response.next_cursor {
        data.insert("nextCursor", next_cursor.as_str());
    }

    if let Some(obj) = crate::nvim::parse::convert_metadata_to_lua_object(response.meta) {
        data.insert("meta", obj);
    }

    data
}
