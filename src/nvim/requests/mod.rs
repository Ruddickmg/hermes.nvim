use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::acp::{Result, error::Error};
use agent_client_protocol::{RequestPermissionOutcome, SelectedPermissionOutcome};
use nvim_oxi::conversion::FromObject;
use tokio::sync::oneshot;
use tracing::warn;
use uuid::Uuid;

#[derive(Debug)]
pub enum Responder {
    Cancelled,
    PermissionResponse(oneshot::Sender<RequestPermissionOutcome>),
}

pub struct Request {
    session_id: String,
    responder: Responder,
}

impl Request {
    pub fn new(session_id: String, responder: Responder) -> Self {
        Self {
            session_id,
            responder,
        }
    }
}

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
    async fn handle_response(&self, request_id: &Uuid, response: nvim_oxi::Object) -> Result<()>;
    async fn cancel_session_requests(&self, session_id: String) -> Result<()>;
    async fn add_request(&self, session_id: String, request_id: Uuid, responder: Responder);
}

impl RequestHandler for Requests {
    async fn add_request(&self, session_id: String, request_id: Uuid, responder: Responder) {
        let mut pending = self.pending.lock().await;
        pending.insert(request_id, Request::new(session_id, responder));
        drop(pending);
    }

    async fn cancel_session_requests(&self, session_id: String) -> Result<()> {
        let mut pending = self.pending.lock().await;
        let cancelled =
            pending
            .extract_if(|_, request| match request.responder {
                Responder::PermissionResponse(_) => request.session_id == session_id,
                _ => false,
            })
            .map(|(id, request)| {
                match request.responder {
                    Responder::PermissionResponse(sender) => {
                        if let Err(e) = sender.send(RequestPermissionOutcome::Cancelled) {
                            return Err(Error::Internal(format!(
                                "Failed to send cancellation for request '{}': {:?}",
                                id, e
                            )));
                        };
                    }
                    _ => panic!("Unexpected responder type when cancelling session requests. This should never happen."),
                };
                Ok((
                    id,
                    Request::new(request.session_id.clone(), Responder::Cancelled),
                ))
            })
            .collect::<Result<Vec<(Uuid, Request)>>>()?;
        pending.extend(cancelled);
        drop(pending);
        Ok(())
    }

    async fn handle_response(&self, request_id: &Uuid, response: nvim_oxi::Object) -> Result<()> {
        let mut pending = self.pending.lock().await;
        let retrieved = pending.remove(request_id);
        drop(pending);
        if let Some(request) = retrieved {
            match request.responder {
                Responder::Cancelled => warn!(
                    "Request was responded to after it was cancelled. request id: '{}', session id: '{}'",
                    request_id, request.session_id
                ),
                Responder::PermissionResponse(sender) => {
                    let option_id: String = String::from_object(response)
                        .map_err(|e| Error::Internal(e.to_string()))?;
                    let outcome = RequestPermissionOutcome::Selected(
                        SelectedPermissionOutcome::new(option_id),
                    );
                    sender.send(outcome).map_err(|e| {
                        Error::Internal(format!(
                            "Failed to send response for request '{}': {:?}",
                            request_id, e
                        ))
                    })?;
                }
            };
            Ok(())
        } else {
            Err(Error::Internal(format!(
                "No pending request found for ID: '{}'",
                request_id
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::RequestPermissionOutcome;

    #[tokio::test]
    async fn test_add_request_increments_pending_count() {
        let requests = Requests::new();
        let session_id = String::from("test-session");
        let request_id = Uuid::new_v4();
        let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
        let responder = Responder::PermissionResponse(sender);

        requests
            .add_request(session_id, request_id, responder)
            .await;

        let pending = requests.pending.lock().await;
        assert_eq!(pending.len(), 1);
        drop(pending);
    }

    #[tokio::test]
    async fn test_handle_response_success() {
        let requests = Requests::new();
        let session_id = String::from("test-session");
        let request_id = Uuid::new_v4();
        let (sender, _) = oneshot::channel::<RequestPermissionOutcome>();
        let responder = Responder::PermissionResponse(sender);

        requests.add_request(session_id, request_id, responder).await;

        let response_obj = nvim_oxi::Object::from("selected-option-id");
        let result = requests
            .handle_response(&request_id, response_obj)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_response_outcome_contains_option_id() {
        let requests = Requests::new();
        let session_id = String::from("test-session");
        let request_id = Uuid::new_v4();
        let (sender, mut receiver) = oneshot::channel::<RequestPermissionOutcome>();
        let responder = Responder::PermissionResponse(sender);

        requests.add_request(session_id, request_id, responder).await;

        let response_obj = nvim_oxi::Object::from("selected-option-id");
        requests.handle_response(&request_id, response_obj).await.unwrap();

        let outcome = receiver.try_recv().expect("Should receive outcome");
        match outcome {
            RequestPermissionOutcome::Selected(selected) => {
                assert_eq!(selected.option_id.0.as_ref(), "selected-option-id");
            }
            _ => panic!("Expected Selected outcome"),
        }
    }

    #[tokio::test]
    async fn test_handle_response_not_found_returns_error() {
        let requests = Requests::new();
        let request_id = Uuid::new_v4();
        let response_obj = nvim_oxi::Object::from("some-option");

        let result = requests.handle_response(&request_id, response_obj).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_response_not_found_error_message() {
        let requests = Requests::new();
        let request_id = Uuid::new_v4();
        let response_obj = nvim_oxi::Object::from("some-option");

        let result = requests.handle_response(&request_id, response_obj).await;

        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("No pending request found"));
    }

    #[tokio::test]
    async fn test_cancel_session_requests_returns_ok() {
        let requests = Requests::new();
        let session_id = String::from("test-session");
        let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();

        requests.add_request(
            session_id.clone(),
            Uuid::new_v4(),
            Responder::PermissionResponse(sender),
        );

        let result = requests.cancel_session_requests(session_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cancel_session_requests_preserves_cancelled_responder() {
        let requests = Requests::new();
        let session_id = String::from("test-session");
        let request_id = Uuid::new_v4();
        let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();

        requests.add_request(
            session_id.clone(),
            request_id,
            Responder::PermissionResponse(sender),
        ).await;

        requests.cancel_session_requests(session_id).await.unwrap();

        let pending = requests.pending.lock().await;
        match pending.get(&request_id).unwrap().responder {
            Responder::Cancelled => {}
            _ => panic!("Request should be Cancelled"),
        }
        drop(pending);
    }

    #[tokio::test]
    async fn test_cancel_session_requests_no_matches_returns_ok() {
        let requests = Requests::new();
        let session_id = String::from("nonexistent-session");

        let result = requests.cancel_session_requests(session_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cancel_session_requests_only_affects_target_session() {
        let requests = Requests::new();
        let session_id = String::from("target-session");
        let other_session_id = String::from("other-session");
        let (target_sender, mut target_receiver) = oneshot::channel::<RequestPermissionOutcome>();
        let (other_sender, mut other_receiver) = oneshot::channel::<RequestPermissionOutcome>();

        requests.add_request(
            session_id.clone(),
            Uuid::new_v4(),
            Responder::PermissionResponse(target_sender),
        ).await;
        requests.add_request(
            other_session_id.clone(),
            Uuid::new_v4(),
            Responder::PermissionResponse(other_sender),
        ).await;

        requests.cancel_session_requests(session_id).await.unwrap();

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
