//! Mock implementation of RequestHandler trait for testing
use agent_client_protocol::schema::v1::RequestId;
use async_trait::async_trait;
use hermes::nvim::requests::{Request, RequestHandler, Responder};
use uuid::Uuid;

#[derive(Clone)]
pub struct MockRequestHandler;

impl Default for MockRequestHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl MockRequestHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait(?Send)]
impl RequestHandler for MockRequestHandler {
    async fn default_response(
        &self,
        _request_id: &Uuid,
        _data: serde_json::Value,
    ) -> hermes::acp::Result<()> {
        Ok(())
    }

    async fn handle_response(
        &self,
        _request_id: &Uuid,
        _response: nvim_oxi::Object,
    ) -> hermes::acp::Result<()> {
        Ok(())
    }

    async fn cancel_session_requests(&self, _session_id: String) -> hermes::acp::Result<()> {
        Ok(())
    }

    async fn add_request(
        &self,
        _session_id: String,
        _agent_request_id: RequestId,
        _responder: Responder,
    ) -> Uuid {
        Uuid::new_v4()
    }

    async fn remove_request(&self, request_id: &Uuid) -> hermes::acp::Result<Request> {
        Err(hermes::acp::error::Error::InvalidInput(format!(
            "No request found for {}",
            request_id
        )))
    }

    async fn get_request(&self, _request_id: &Uuid) -> Option<Request> {
        None
    }
}
