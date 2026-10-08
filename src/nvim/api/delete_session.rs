use futures::future;
use nvim_oxi::{
    Dictionary, Object, ObjectKind,
    conversion::FromObject,
    lua::{self, Poppable, Pushable},
};
use tracing::error;

use agent_client_protocol::schema::v1::{CancelNotification, DeleteSessionRequest, SessionId};

use crate::{
    acp::{self, error::Error},
    api::Api,
    nvim::{configuration::dict_from_object, requests::RequestHandler},
};

#[derive(Clone, Debug)]
pub enum DeleteSessionArg {
    Multiple(Vec<String>),
    Single(String),
}

const EXPECTED: &str = "String or Array of Strings";

impl FromObject for DeleteSessionArg {
    fn from_object(obj: Object) -> Result<Self, nvim_oxi::conversion::Error> {
        match obj.kind() {
            ObjectKind::String => {
                let session_id = unsafe { obj.into_string_unchecked() };
                Ok(Self::Single(session_id.to_string()))
            }
            ObjectKind::Array => {
                let sessions = unsafe { obj.into_array_unchecked() };
                Ok(Self::Multiple(
                    sessions
                        .into_iter()
                        .map(|obj| {
                            if let ObjectKind::String = obj.kind() {
                                Ok(unsafe { obj.into_string_unchecked() })
                            } else {
                                Err(nvim_oxi::conversion::Error::FromWrongType {
                                    expected: EXPECTED,
                                    actual: obj.kind().as_static(),
                                })
                            }
                        })
                        .collect::<Result<Vec<nvim_oxi::String>, nvim_oxi::conversion::Error>>()?
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect(),
                ))
            }
            other => Err(nvim_oxi::conversion::Error::FromWrongType {
                expected: EXPECTED,
                actual: other.as_static(),
            }),
        }
    }
}

impl Poppable for DeleteSessionArg {
    unsafe fn pop(lua: *mut nvim_oxi::lua::ffi::State) -> Result<Self, lua::Error> {
        let obj = unsafe { Object::pop(lua)? };
        Ok(Self::from_object(obj)
            .inspect_err(|e| {
                error!(
                    "An error occurred while parsing the delete_session arguments: {:?}",
                    e
                )
            })
            .unwrap_or(Self::Multiple(vec![])))
    }
}

impl Pushable for DeleteSessionArg {
    unsafe fn push(self, state: *mut lua::ffi::State) -> Result<i32, lua::Error> {
        unsafe {
            match self {
                Self::Single(s) => s.push(state),
                Self::Multiple(vec) => vec.push(state),
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct DeleteSessionOptions {
    pub cancel: bool,
}

impl Default for DeleteSessionOptions {
    fn default() -> Self {
        Self { cancel: true }
    }
}

impl FromObject for DeleteSessionOptions {
    fn from_object(obj: Object) -> std::result::Result<Self, nvim_oxi::conversion::Error> {
        if obj.is_nil() {
            return Ok(Self::default());
        }

        let dict = dict_from_object(obj)?;

        let cancel = dict
            .get("cancel")
            .and_then(|o| {
                if matches!(o.kind(), ObjectKind::Boolean) {
                    Some(unsafe { o.as_boolean_unchecked() })
                } else {
                    None
                }
            })
            .unwrap_or(true);

        Ok(Self { cancel })
    }
}

impl Poppable for DeleteSessionOptions {
    unsafe fn pop(lua_state: *mut lua::ffi::State) -> std::result::Result<Self, lua::Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        Ok(Self::from_object(obj)
            .inspect_err(|e| error!("Error parsing delete_session options: {:?}", e))
            .unwrap_or_default())
    }
}

impl Pushable for DeleteSessionOptions {
    unsafe fn push(self, lua_state: *mut lua::ffi::State) -> std::result::Result<i32, lua::Error> {
        let mut dict = Dictionary::new();
        dict.insert("cancel", self.cancel);
        unsafe { Object::from(dict).push(lua_state) }
    }
}

pub type DeleteSessionArgs = (DeleteSessionArg, Option<DeleteSessionOptions>);

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn delete_session(&self, (args, options): DeleteSessionArgs) -> acp::Result<()> {
        let state = self.state.lock().await;
        let agent_info = state.agent_info.clone();
        drop(state);

        if !agent_info.can_delete_session() {
            return Ok(());
        }

        let session_ids: Vec<String> = match args {
            DeleteSessionArg::Single(id) => vec![id],
            DeleteSessionArg::Multiple(ids) => ids,
        };

        let connection = self
            .connection_manager
            .get_current_connection()
            .await
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;
        let request_handler = &*self.request_handler;

        let cancel_enabled = options.map(|o| o.cancel).unwrap_or(true);

        let futures = session_ids
            .into_iter()
            .map(|session_id| {
                let delete = DeleteSessionRequest::new(SessionId::from(session_id.clone()));
                async move {
                    if cancel_enabled {
                        let cancel = CancelNotification::new(SessionId::from(session_id.clone()));
                        connection.cancel(cancel).await?;
                        request_handler.cancel_session_requests(session_id).await?;
                    }
                    connection.delete_session(delete).await
                }
            })
            .collect::<Vec<_>>();

        future::try_join_all(futures).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_session_id() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("test-session".to_string()),
            "[a-zA-Z0-9_-]{1,20}".prop_map(|s| s.to_string())
        ]
    }

    fn arb_delete_session_arg() -> impl Strategy<Value = DeleteSessionArg> {
        prop_oneof![
            arb_session_id().prop_map(DeleteSessionArg::Single),
            prop::collection::vec(arb_session_id(), 0..5).prop_map(DeleteSessionArg::Multiple),
        ]
    }

    proptest! {
        #[test]
        fn test_delete_session_arg_roundtrip(arg in arb_delete_session_arg()) {
            match arg {
                DeleteSessionArg::Single(ref id) => {
                    prop_assert!(!id.is_empty(), "Single session ID should not be empty");
                }
                DeleteSessionArg::Multiple(ref ids) => {
                    prop_assert!(ids.len() <= 5, "Multiple should have at most 5 session IDs");
                }
            }
        }
    }

    #[test]
    fn test_delete_session_arg_from_object_string() {
        let obj = Object::from("test-session-id");
        let result = DeleteSessionArg::from_object(obj).unwrap();
        match result {
            DeleteSessionArg::Single(ref id) => assert_eq!(id, "test-session-id"),
            _ => panic!("Expected Single variant"),
        }
    }

    #[test]
    fn test_delete_session_arg_from_object_array() {
        let arr = nvim_oxi::Array::from_iter(vec![Object::from("a"), Object::from("b")]);
        let obj = Object::from(arr);
        let result = DeleteSessionArg::from_object(obj).unwrap();
        match result {
            DeleteSessionArg::Multiple(ref ids) => {
                assert_eq!(ids.as_slice(), &["a", "b"]);
            }
            _ => panic!("Expected Multiple variant"),
        }
    }

    #[test]
    fn test_delete_session_arg_from_object_invalid_type() {
        let obj = Object::from(42);
        let result = DeleteSessionArg::from_object(obj);
        assert!(
            result.is_err(),
            "Expected error for non-string, non-array input"
        );
    }

    #[test]
    fn test_delete_session_options_default_cancel() {
        let options = DeleteSessionOptions::default();
        assert!(options.cancel, "Cancel should default to true");
    }

    #[test]
    fn test_delete_session_options_from_empty_dict() {
        let dict = Dictionary::new();
        let obj = Object::from(dict);
        let options = DeleteSessionOptions::from_object(obj).unwrap();
        assert!(
            options.cancel,
            "Cancel should default to true when not specified"
        );
    }

    #[test]
    fn test_delete_session_options_cancel_false() {
        let mut dict = Dictionary::new();
        dict.insert("cancel", false);
        let obj = Object::from(dict);
        let options = DeleteSessionOptions::from_object(obj).unwrap();
        assert!(!options.cancel, "Cancel should be false when set to false");
    }

    #[test]
    fn test_delete_session_options_from_nil() {
        let obj = Object::nil();
        let result = DeleteSessionOptions::from_object(obj).unwrap();
        assert!(result.cancel, "Cancel should default to true from nil");
    }
}
