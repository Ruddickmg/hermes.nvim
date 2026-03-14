pub mod request;
pub mod responder;
use crate::{
    acp::{error::Error, Result},
    nvim::requests::request::Request,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

pub use responder::*;

pub struct Requests {
    pending: Arc<Mutex<HashMap<Uuid, Request>>>,
}

impl Requests {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Requests {
    fn default() -> Self {
        Self {
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

pub trait RequestHandler {
    fn default_response(&self, request_id: &Uuid, data: serde_json::Value) -> Result<()>;
    fn handle_response(&self, request_id: &Uuid, response: nvim_oxi::Object) -> Result<()>;
    fn cancel_session_requests(&self, session_id: String) -> Result<()>;
    fn add_request(&self, session_id: String, request_id: Uuid, responder: Responder);
}

impl RequestHandler for Requests {
    fn default_response(&self, request_id: &Uuid, data: serde_json::Value) -> Result<()> {
        let mut pending = self.pending.blocking_lock();
        let retrieved = pending.remove(request_id);
        drop(pending);
        if let Some(request) = retrieved {
            request.default(data)
        } else {
            Err(Error::Internal(format!(
                "No pending request found for ID: '{}'",
                request_id
            )))
        }
    }
    fn add_request(&self, session_id: String, request_id: Uuid, responder: Responder) {
        let mut pending = self.pending.blocking_lock();
        pending.insert(request_id, Request::new(session_id, responder));
        drop(pending);
    }

    fn cancel_session_requests(&self, session_id: String) -> Result<()> {
        let mut pending = self.pending.blocking_lock();
        let cancelled = pending
            .extract_if(|_, request| {
                request.is_permission_request() && request.is_session(session_id.clone())
            })
            .map(|(id, mut request)| {
                request.cancel()?;
                Ok((id, Request::new(session_id.clone(), Responder::Cancelled)))
            })
            .collect::<Result<Vec<(Uuid, Request)>>>()?;
        // TODO: figure out a solution for cleaning up cancelled requests (potential memory leak)
        pending.extend(cancelled);
        drop(pending);
        Ok(())
    }

    fn handle_response(&self, request_id: &Uuid, response: nvim_oxi::Object) -> Result<()> {
        let mut pending = self.pending.blocking_lock();
        let retrieved = pending.remove(request_id);
        drop(pending);

        match retrieved {
            Some(mut request) => {
                request
                    .respond(response)
                    .map_err(|e| Error::Internal(format!("Failed to respond to request: {}", e)))?;
                Ok(())
            }
            None => Err(Error::Internal(format!(
                "No pending request found for ID: '{}'",
                request_id
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::{
        RequestPermissionOutcome, RequestPermissionRequest, SessionId, ToolCallUpdate,
    };
    use pretty_assertions::assert_eq;
    use tokio::sync::oneshot;

    fn create_test_permission_request(session_id: impl Into<String>) -> RequestPermissionRequest {
        use agent_client_protocol::{ToolCallId, ToolCallUpdateFields};
        RequestPermissionRequest::new(
            SessionId::from(session_id.into()),
            ToolCallUpdate::new(
                ToolCallId::from("test-call-id"),
                ToolCallUpdateFields::default(),
            ),
            vec![],
        )
    }

    #[test]
    fn test_handle_response_success() {
        let requests = Requests::new();
        let session_id = String::from("test-session");
        let request_id = Uuid::new_v4();
        let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
        let responder =
            Responder::PermissionResponse(sender, create_test_permission_request("test-session"));

        requests.add_request(session_id, request_id, responder);

        let response_obj = nvim_oxi::Object::from("selected-option-id");
        let result = requests.handle_response(&request_id, response_obj);

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_response_outcome_contains_option_id() {
        let requests = Requests::new();
        let session_id = String::from("test-session");
        let request_id = Uuid::new_v4();
        let (sender, mut receiver) = oneshot::channel::<RequestPermissionOutcome>();
        let responder =
            Responder::PermissionResponse(sender, create_test_permission_request("test-session"));

        requests.add_request(session_id, request_id, responder);

        let response_obj = nvim_oxi::Object::from("selected-option-id");
        requests.handle_response(&request_id, response_obj).unwrap();

        let outcome = receiver.try_recv().expect("Should receive outcome");
        match outcome {
            RequestPermissionOutcome::Selected(selected) => {
                assert_eq!(selected.option_id.0.as_ref(), "selected-option-id");
            }
            _ => panic!("Expected Selected outcome"),
        }
    }

    #[test]
    fn test_handle_response_not_found_returns_error() {
        let requests = Requests::new();
        let request_id = Uuid::new_v4();
        let response_obj = nvim_oxi::Object::from("some-option");

        let result = requests.handle_response(&request_id, response_obj);

        assert!(result.is_err());
    }

    #[test]
    fn test_handle_response_not_found_error_message() {
        let requests = Requests::new();
        let request_id = Uuid::new_v4();
        let response_obj = nvim_oxi::Object::from("some-option");

        let result = requests.handle_response(&request_id, response_obj);

        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("No pending request found"));
    }

    #[test]
    fn test_cancel_session_requests_returns_ok() {
        let requests = Requests::new();
        let session_id = String::from("test-session");
        let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();

        requests.add_request(
            session_id.clone(),
            Uuid::new_v4(),
            Responder::PermissionResponse(sender, create_test_permission_request("test-session")),
        );

        let result = requests.cancel_session_requests(session_id);
        assert!(result.is_ok());
    }

    /*
    #[test]
    fn test_cancel_session_requests_preserves_cancelled_responder() {
        let requests = Requests::new();
        let session_id = String::from("test-session");
        let request_id = Uuid::new_v4();
        let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();

        requests.add_request(
            session_id.clone(),
            request_id,
            Responder::PermissionResponse(sender, create_test_permission_request("test-session")),
        );

        requests.cancel_session_requests(session_id).unwrap();

        let pending = requests.pending.blocking_lock();
        match pending.get(&request_id).unwrap().responder.unwrap() {
            Responder::Cancelled => {}
            _ => panic!("Request should be Cancelled"),
        }
        drop(pending);
    }
    */

    #[test]
    fn test_cancel_session_requests_no_matches_returns_ok() {
        let requests = Requests::new();
        let session_id = String::from("nonexistent-session");

        let result = requests.cancel_session_requests(session_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cancel_session_requests_only_affects_target_session() {
        let requests = Requests::new();
        let session_id = String::from("target-session");
        let other_session_id = String::from("other-session");
        let (target_sender, mut target_receiver) = oneshot::channel::<RequestPermissionOutcome>();
        let (other_sender, mut other_receiver) = oneshot::channel::<RequestPermissionOutcome>();

        requests.add_request(
            session_id.clone(),
            Uuid::new_v4(),
            Responder::PermissionResponse(
                target_sender,
                create_test_permission_request("target-session"),
            ),
        );
        requests.add_request(
            other_session_id.clone(),
            Uuid::new_v4(),
            Responder::PermissionResponse(
                other_sender,
                create_test_permission_request("other-session"),
            ),
        );

        requests.cancel_session_requests(session_id).unwrap();

        // Target session should be cancelled
        let target_outcome = target_receiver
            .try_recv()
            .expect("Should receive cancellation");
        assert_eq!(target_outcome, RequestPermissionOutcome::Cancelled);

        // Other session should not be cancelled
        let other_outcome = other_receiver.try_recv();
        assert!(other_outcome.is_err());
    }
}
