//! Mock implementation of RequestHandler trait for testing
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

impl RequestHandler for MockRequestHandler {
    fn default_response(
        &self,
        _request_id: &Uuid,
        _data: serde_json::Value,
    ) -> hermes::acp::Result<()> {
        Ok(())
    }

    fn handle_response(
        &self,
        _request_id: &Uuid,
        _response: nvim_oxi::Object,
    ) -> hermes::acp::Result<()> {
        Ok(())
    }

    fn cancel_session_requests(&self, _session_id: String) -> hermes::acp::Result<()> {
        Ok(())
    }

    fn add_request(&self, _session_id: String, _responder: Responder) -> Uuid {
        Uuid::new_v4()
    }

    fn get_request(&self, _request_id: &Uuid) -> Option<Request> {
        None
    }
}
