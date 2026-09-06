//! Integration tests for elicitation request handling via Request
use agent_client_protocol::schema::v1::{
    CreateElicitationRequest, CreateElicitationResponse, ElicitationAcceptAction,
    ElicitationAction, ElicitationContentValue, ElicitationFormMode, ElicitationPropertySchema,
    ElicitationSchema, ElicitationScope, ElicitationSessionScope, OtherElicitationPropertySchema,
    StringPropertySchema,
};
use async_lock::Mutex;
use hermes::nvim::requests::{RequestHandler, Requests, Responder};
use hermes::nvim::state::PluginState;
use hermes::utilities::NvimRuntime;
use nvim_oxi::Object;
use pretty_assertions::assert_eq;
use std::sync::Arc;

fn mock_runtime() -> NvimRuntime {
    NvimRuntime::new()
}

/// Helper to block on an async future in synchronous tests
fn block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future,
{
    futures::executor::block_on(fut)
}

fn create_elicitation_request_with(schema: ElicitationSchema) -> CreateElicitationRequest {
    let scope = ElicitationScope::Session(ElicitationSessionScope::new("test-session"));
    let mode = ElicitationFormMode::new(scope, schema);
    CreateElicitationRequest::new(mode, "Please enter your name")
}

fn create_elicitation_request() -> CreateElicitationRequest {
    create_elicitation_request_with(ElicitationSchema::new().string("name", true))
}

/// Adds an elicitation request to the Requests registry and returns its ID plus a
/// receiver for the pending response.
fn add_elicitation_request_with(
    requests: &Arc<Requests>,
    request: CreateElicitationRequest,
) -> (
    uuid::Uuid,
    async_channel::Receiver<CreateElicitationResponse>,
) {
    let (sender, receiver) = async_channel::bounded::<CreateElicitationResponse>(1);
    let responder = Responder::Elicitation(sender, request);
    let request_id = block_on(requests.add_request("test-session".to_string(), responder));
    (request_id, receiver)
}

/// Adds an elicitation request with the default string schema.
fn add_elicitation_request(
    requests: &Arc<Requests>,
) -> (
    uuid::Uuid,
    async_channel::Receiver<CreateElicitationResponse>,
) {
    add_elicitation_request_with(requests, create_elicitation_request())
}

#[nvim_oxi::test]
fn elicitation_response_accept_delivers_response() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let (request_id, receiver) = add_elicitation_request(&requests);

    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("accept"));
    let response = Object::from(dict);

    block_on(requests.handle_response(&request_id, response))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let received = receiver.try_recv().expect("Should receive accept response");
    assert_eq!(
        received.action,
        ElicitationAction::Accept(ElicitationAcceptAction::new())
    );
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_decline_delivers_response() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let (request_id, receiver) = add_elicitation_request(&requests);

    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("decline"));
    let response = Object::from(dict);

    block_on(requests.handle_response(&request_id, response))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let received = receiver
        .try_recv()
        .expect("Should receive decline response");
    assert_eq!(received.action, ElicitationAction::Decline);
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_accept_with_content_delivers_content() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let (request_id, receiver) = add_elicitation_request(&requests);

    let mut content = nvim_oxi::Dictionary::default();
    content.insert("name", Object::from("Alice"));
    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("accept"));
    dict.insert("content", Object::from(content));
    let response = Object::from(dict);

    block_on(requests.handle_response(&request_id, response))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let received = receiver.try_recv().expect("Should receive accept response");
    let mut expected = std::collections::BTreeMap::new();
    expected.insert(
        "name".to_string(),
        ElicitationContentValue::String("Alice".to_string()),
    );
    assert_eq!(
        received.action,
        ElicitationAction::Accept(ElicitationAcceptAction::new().content(expected))
    );
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_cancel_delivers_response() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let (request_id, receiver) = add_elicitation_request(&requests);

    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("cancel"));
    let response = Object::from(dict);

    block_on(requests.handle_response(&request_id, response))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let received = receiver.try_recv().expect("Should receive cancel response");
    assert_eq!(received.action, ElicitationAction::Cancel);
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_invalid_action_returns_error() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let (request_id, _receiver) = add_elicitation_request(&requests);

    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("not_a_real_action"));
    let response = Object::from(dict);

    let result = block_on(requests.handle_response(&request_id, response));
    assert!(result.is_err(), "Invalid action should return an error");
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_default_response_cancels_when_no_listener() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let (request_id, receiver) = add_elicitation_request(&requests);

    block_on(requests.default_response(&request_id, serde_json::Value::Null))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let received = receiver
        .try_recv()
        .expect("Should receive default response");
    assert_eq!(received.action, ElicitationAction::Cancel);
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_accept_with_invalid_content_type_returns_error() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    // The schema declares `name` as a string.
    let (request_id, _receiver) = add_elicitation_request(&requests);

    let mut content = nvim_oxi::Dictionary::default();
    content.insert("name", Object::from(123i64));
    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("accept"));
    dict.insert("content", Object::from(content));
    let response = Object::from(dict);

    let result = block_on(requests.handle_response(&request_id, response));
    assert!(
        result.is_err(),
        "Type-mismatched content should return an error"
    );
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_accept_missing_required_field_returns_error() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    // The schema requires the `name` field.
    let (request_id, _receiver) = add_elicitation_request(&requests);

    let content = nvim_oxi::Dictionary::default();
    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("accept"));
    dict.insert("content", Object::from(content));
    let response = Object::from(dict);

    let result = block_on(requests.handle_response(&request_id, response));
    assert!(
        result.is_err(),
        "Missing required field should return an error"
    );
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_missing_action_returns_error() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let (request_id, _receiver) = add_elicitation_request(&requests);

    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("content", Object::from("unused"));
    let response = Object::from(dict);

    let result = block_on(requests.handle_response(&request_id, response));
    assert!(result.is_err(), "Missing action should return an error");
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_accept_whole_float_coerces_to_integer() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let schema = ElicitationSchema::new().integer("age", 0, i64::MAX, true);
    let request = create_elicitation_request_with(schema);
    let (request_id, receiver) = add_elicitation_request_with(&requests, request);

    let mut content = nvim_oxi::Dictionary::default();
    content.insert("age", Object::from(5.0f64));
    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("accept"));
    dict.insert("content", Object::from(content));
    let response = Object::from(dict);

    block_on(requests.handle_response(&request_id, response))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let received = receiver.try_recv().expect("Should receive accept response");
    let mut expected = std::collections::BTreeMap::new();
    expected.insert("age".to_string(), ElicitationContentValue::Integer(5));
    assert_eq!(
        received.action,
        ElicitationAction::Accept(ElicitationAcceptAction::new().content(expected))
    );
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_accept_rejects_string_for_integer() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let schema = ElicitationSchema::new().integer("age", 0, i64::MAX, true);
    let request = create_elicitation_request_with(schema);
    let (request_id, _receiver) = add_elicitation_request_with(&requests, request);

    let mut content = nvim_oxi::Dictionary::default();
    content.insert("age", Object::from("abc"));
    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("accept"));
    dict.insert("content", Object::from(content));
    let response = Object::from(dict);

    let result = block_on(requests.handle_response(&request_id, response));
    assert!(
        result.is_err(),
        "String value for integer schema should return an error"
    );
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_accept_rejects_string_for_boolean() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let schema = ElicitationSchema::new().boolean("flag", true);
    let request = create_elicitation_request_with(schema);
    let (request_id, _receiver) = add_elicitation_request_with(&requests, request);

    let mut content = nvim_oxi::Dictionary::default();
    content.insert("flag", Object::from("yes"));
    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("accept"));
    dict.insert("content", Object::from(content));
    let response = Object::from(dict);

    let result = block_on(requests.handle_response(&request_id, response));
    assert!(
        result.is_err(),
        "String value for boolean schema should return an error"
    );
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_accept_ignores_unknown_content_key() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let (request_id, receiver) = add_elicitation_request(&requests);

    let mut content = nvim_oxi::Dictionary::default();
    content.insert("name", Object::from("Alice"));
    content.insert("extra", Object::from("ignored"));
    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("accept"));
    dict.insert("content", Object::from(content));
    let response = Object::from(dict);

    block_on(requests.handle_response(&request_id, response))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let received = receiver.try_recv().expect("Should receive accept response");
    let mut expected = std::collections::BTreeMap::new();
    expected.insert(
        "name".to_string(),
        ElicitationContentValue::String("Alice".to_string()),
    );
    assert_eq!(
        received.action,
        ElicitationAction::Accept(ElicitationAcceptAction::new().content(expected))
    );
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_accept_rejects_invalid_enum_string() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let schema = ElicitationSchema::new().property(
        "color",
        ElicitationPropertySchema::String(
            StringPropertySchema::new().enum_values(vec!["red".to_string(), "blue".to_string()]),
        ),
        true,
    );
    let request = create_elicitation_request_with(schema);
    let (request_id, _receiver) = add_elicitation_request_with(&requests, request);

    let mut content = nvim_oxi::Dictionary::default();
    content.insert("color", Object::from("green"));
    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("accept"));
    dict.insert("content", Object::from(content));
    let response = Object::from(dict);

    let result = block_on(requests.handle_response(&request_id, response));
    assert!(
        result.is_err(),
        "Non-enum string value should return an error"
    );
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_accept_other_rejects_when_configured() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    block_on(async {
        state
            .lock()
            .await
            .config
            .permissions
            .elicitation
            .reject_unknown_elicitation_values = true;
    });
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let schema = ElicitationSchema::new().property(
        "fut",
        ElicitationPropertySchema::Other(OtherElicitationPropertySchema::new(
            "future",
            std::collections::BTreeMap::new(),
        )),
        false,
    );
    let request = create_elicitation_request_with(schema);
    let (request_id, _receiver) = add_elicitation_request_with(&requests, request);

    let mut content = nvim_oxi::Dictionary::default();
    content.insert("fut", Object::from("value"));
    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("accept"));
    dict.insert("content", Object::from(content));
    let response = Object::from(dict);

    let result = block_on(requests.handle_response(&request_id, response));
    assert!(
        result.is_err(),
        "Other property should be rejected when configured"
    );
    Ok(())
}

#[nvim_oxi::test]
fn elicitation_response_accept_other_passes_when_not_rejected() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests =
        Arc::new(Requests::new(mock_runtime(), state.clone()).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let schema = ElicitationSchema::new().property(
        "fut",
        ElicitationPropertySchema::Other(OtherElicitationPropertySchema::new(
            "future",
            std::collections::BTreeMap::new(),
        )),
        false,
    );
    let request = create_elicitation_request_with(schema);
    let (request_id, receiver) = add_elicitation_request_with(&requests, request);

    let mut content = nvim_oxi::Dictionary::default();
    content.insert("fut", Object::from("value"));
    let mut dict = nvim_oxi::Dictionary::default();
    dict.insert("action", Object::from("accept"));
    dict.insert("content", Object::from(content));
    let response = Object::from(dict);

    block_on(requests.handle_response(&request_id, response))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let received = receiver.try_recv().expect("Should receive accept response");
    let mut expected = std::collections::BTreeMap::new();
    expected.insert(
        "fut".to_string(),
        ElicitationContentValue::String("value".to_string()),
    );
    assert_eq!(
        received.action,
        ElicitationAction::Accept(ElicitationAcceptAction::new().content(expected))
    );
    Ok(())
}
