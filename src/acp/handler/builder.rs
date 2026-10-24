use std::sync::Arc;

use agent_client_protocol::{
    self as acp, Client, ConnectionTo, Responder, on_receive_notification, on_receive_request,
    schema::v1::{
        CompleteElicitationNotification, CreateElicitationRequest, CreateElicitationResponse,
        CreateTerminalRequest, CreateTerminalResponse, ReadTextFileRequest, ReadTextFileResponse,
        ReleaseTerminalRequest, ReleaseTerminalResponse, RequestPermissionRequest,
        RequestPermissionResponse, SessionNotification, TerminalOutputRequest,
        TerminalOutputResponse, WaitForTerminalExitRequest, WaitForTerminalExitResponse,
        WriteTextFileRequest, WriteTextFileResponse,
    },
};
use tracing::error;

use crate::Handler;

macro_rules! spawn_request_handler {
    ($handler:expr, $method:ident, $request:ty, $response:ty) => {{
        let handler = $handler.clone();
        move |request: $request, responder: Responder<$response>, cx: ConnectionTo<acp::Agent>| {
            let handler = handler.clone();
            let request_id = responder.id().clone();
            let cancellation = responder.cancellation();
            async move {
                cx.spawn(async move {
                    let response = cancellation
                        .run_until_cancelled(handler.$method(request, request_id.clone()))
                        .await;
                    if cancellation.is_cancelled()
                        && let Err(error) = handler.cancel_request(&request_id).await
                    {
                        error!("Failed to clean up cancelled request: {}", error);
                    }
                    responder.respond_with_result(response)
                })?;
                Ok(())
            }
        }
    }};
}

pub fn build_client(
    handler: Arc<Handler>,
) -> acp::Builder<
    Client,
    impl acp::HandleDispatchFrom<acp::Agent>,
    impl acp::RunWithConnectionTo<acp::Agent>,
> {
    Client
        .builder()
        .name("hermes")
        .on_receive_request(
            spawn_request_handler!(
                handler,
                request_permission,
                RequestPermissionRequest,
                RequestPermissionResponse
            ),
            on_receive_request!(),
        )
        .on_receive_request(
            spawn_request_handler!(
                handler,
                write_text_file,
                WriteTextFileRequest,
                WriteTextFileResponse
            ),
            on_receive_request!(),
        )
        .on_receive_request(
            spawn_request_handler!(
                handler,
                read_text_file,
                ReadTextFileRequest,
                ReadTextFileResponse
            ),
            on_receive_request!(),
        )
        .on_receive_request(
            spawn_request_handler!(
                handler,
                create_terminal,
                CreateTerminalRequest,
                CreateTerminalResponse
            ),
            on_receive_request!(),
        )
        .on_receive_request(
            spawn_request_handler!(
                handler,
                terminal_output,
                TerminalOutputRequest,
                TerminalOutputResponse
            ),
            on_receive_request!(),
        )
        .on_receive_request(
            spawn_request_handler!(
                handler,
                wait_for_terminal_exit,
                WaitForTerminalExitRequest,
                WaitForTerminalExitResponse
            ),
            on_receive_request!(),
        )
        .on_receive_request(
            spawn_request_handler!(
                handler,
                release_terminal,
                ReleaseTerminalRequest,
                ReleaseTerminalResponse
            ),
            on_receive_request!(),
        )
        .on_receive_notification(
            {
                let handler = handler.clone();
                move |notif: SessionNotification, _cx: ConnectionTo<acp::Agent>| {
                    let handler = handler.clone();
                    async move { handler.session_notification(notif).await }
                }
            },
            on_receive_notification!(),
        )
        .on_receive_request(
            spawn_request_handler!(
                handler,
                create_elicitation,
                CreateElicitationRequest,
                CreateElicitationResponse
            ),
            on_receive_request!(),
        )
        .on_receive_notification(
            {
                let handler = handler.clone();
                move |notif: CompleteElicitationNotification, _cx: ConnectionTo<acp::Agent>| {
                    let handler = handler.clone();
                    async move { handler.elicitation_complete(notif).await }
                }
            },
            on_receive_notification!(),
        )
}
