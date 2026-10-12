//! Builder factory for the ACP `Client` role.
//!
//! Request/notification handlers are closures registered on a `Client.builder()`.
//! This module wires our existing `Handler` inherent methods into the builder so
//! the dispatch loop calls into them automatically when the agent sends a request
//! or notification.
//!
//! Every closure clones an `Arc<Handler>`, calls the matching method on the
//! handler, and forwards the result back to the responder. Permission gating
//! and autocommand dispatch all stay inside `Handler`; this module is just the
//! glue layer.

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

use crate::Handler;

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
            {
                let handler = handler.clone();
                move |req: RequestPermissionRequest,
                      responder: Responder<RequestPermissionResponse>,
                      _cx: ConnectionTo<acp::Agent>| {
                    let handler = handler.clone();
                    let request_id = responder.id().clone();
                    async move {
                        responder
                            .respond_with_result(handler.request_permission(req, request_id).await)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let handler = handler.clone();
                move |req: WriteTextFileRequest,
                      responder: Responder<WriteTextFileResponse>,
                      _cx: ConnectionTo<acp::Agent>| {
                    let handler = handler.clone();
                    let request_id = responder.id().clone();
                    async move {
                        responder
                            .respond_with_result(handler.write_text_file(req, request_id).await)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let handler = handler.clone();
                move |req: ReadTextFileRequest,
                      responder: Responder<ReadTextFileResponse>,
                      _cx: ConnectionTo<acp::Agent>| {
                    let handler = handler.clone();
                    let request_id = responder.id().clone();
                    async move {
                        responder.respond_with_result(handler.read_text_file(req, request_id).await)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let handler = handler.clone();
                move |req: CreateTerminalRequest,
                      responder: Responder<CreateTerminalResponse>,
                      _cx: ConnectionTo<acp::Agent>| {
                    let handler = handler.clone();
                    let request_id = responder.id().clone();
                    async move {
                        responder
                            .respond_with_result(handler.create_terminal(req, request_id).await)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let handler = handler.clone();
                move |req: TerminalOutputRequest,
                      responder: Responder<TerminalOutputResponse>,
                      _cx: ConnectionTo<acp::Agent>| {
                    let handler = handler.clone();
                    let request_id = responder.id().clone();
                    async move {
                        responder
                            .respond_with_result(handler.terminal_output(req, request_id).await)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let handler = handler.clone();
                move |req: WaitForTerminalExitRequest,
                      responder: Responder<WaitForTerminalExitResponse>,
                      _cx: ConnectionTo<acp::Agent>| {
                    let handler = handler.clone();
                    let request_id = responder.id().clone();
                    async move {
                        responder.respond_with_result(
                            handler.wait_for_terminal_exit(req, request_id).await,
                        )
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let handler = handler.clone();
                move |req: ReleaseTerminalRequest,
                      responder: Responder<ReleaseTerminalResponse>,
                      _cx: ConnectionTo<acp::Agent>| {
                    let handler = handler.clone();
                    let request_id = responder.id().clone();
                    async move {
                        responder
                            .respond_with_result(handler.release_terminal(req, request_id).await)
                    }
                }
            },
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
            {
                let handler = handler.clone();
                move |req: CreateElicitationRequest,
                      responder: Responder<CreateElicitationResponse>,
                      _cx: ConnectionTo<acp::Agent>| {
                    let handler = handler.clone();
                    let request_id = responder.id().clone();
                    async move {
                        responder
                            .respond_with_result(handler.create_elicitation(req, request_id).await)
                    }
                }
            },
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
