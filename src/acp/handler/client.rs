use crate::{
    Handler, acp,
    nvim::{autocommands::Commands, parse, requests::Responder},
};
use agent_client_protocol::{
    Error, Result,
    schema::v1::{
        CompleteElicitationNotification, CreateElicitationRequest, CreateElicitationResponse,
        CreateTerminalRequest, CreateTerminalResponse, ElicitationMode, ElicitationScope,
        ReadTextFileRequest, ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
        RequestPermissionRequest, RequestPermissionResponse, SessionId, SessionNotification,
        SessionUpdate, TerminalExitStatus, TerminalOutputRequest, TerminalOutputResponse,
        WaitForTerminalExitRequest, WaitForTerminalExitResponse, WriteTextFileRequest,
        WriteTextFileResponse,
    },
};
use async_channel::bounded;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HermesNotification {
    pub session_id: SessionId,
    pub prompt_id: String,
    pub update: SessionUpdate,
}

// In 0.10.x these were trait methods on `impl Client for Handler`. In 0.11
// the `Client` trait is gone and handlers are registered as builder closures
// (see `src/acp/handler/builder.rs`). The methods below remain as inherent
// methods on `Handler` so the builder closures can delegate to them.
impl Handler {
    pub async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse> {
        if !self.can_request_permissions().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) =
            bounded::<agent_client_protocol::schema::v1::RequestPermissionOutcome>(1);
        info!("Requesting permission for: {:?}", args);

        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::PermissionRequest,
            args.clone(),
            Responder::PermissionResponse(sender),
        )
        .await?;
        receiver
            .recv()
            .await
            .map_err(|e| {
                error!("{:?}", e);
                Error::internal_error()
            })
            .map(RequestPermissionResponse::new)
    }

    pub async fn create_elicitation(
        &self,
        args: CreateElicitationRequest,
    ) -> Result<CreateElicitationResponse> {
        let (session_id, command) = match &args.mode {
            ElicitationMode::Form(mode) => {
                if !self.can_request_form_elicitation().await {
                    return Err(Error::method_not_found());
                }
                (
                    session_id_from_scope(&mode.scope),
                    Commands::FormElicitation,
                )
            }
            ElicitationMode::Url(mode) => {
                if !self.can_request_url_elicitation().await {
                    return Err(Error::method_not_found());
                }
                (session_id_from_scope(&mode.scope), Commands::UrlElicitation)
            }
            _ => return Err(Error::method_not_found()),
        };

        let (sender, receiver) = bounded::<CreateElicitationResponse>(1);
        info!("Requesting elicitation: {:?}", args);

        self.execute_autocommand_request(
            session_id,
            command,
            args.clone(),
            Responder::Elicitation(sender, args),
        )
        .await?;
        let resp = receiver
            .recv()
            .await
            .map_err(|e| {
                error!("{:?}", e);
                Error::internal_error()
            })
            .map(CreateElicitationResponse::from);
        resp
    }

    pub async fn elicitation_complete(
        &self,
        notification: CompleteElicitationNotification,
    ) -> Result<()> {
        if !self.can_request_url_elicitation().await {
            return Err(Error::method_not_found());
        }
        info!("Elicitation complete: {:?}", notification);
        Ok(self
            .execute_autocommand(Commands::ElicitationComplete, notification)
            .await?)
    }

    /// Shared notification processing logic.
    ///
    /// Maps a `SessionUpdate` to the corresponding `Commands` variant, generates
    /// prompt IDs for user messages, and builds the `HermesNotification` payload.
    ///
    /// Does **not** write to history or fire autocommands — callers handle those.
    async fn process_notification(
        &self,
        session_notification: &SessionNotification,
    ) -> Result<(Commands, HermesNotification)> {
        let session_id = session_notification.session_id.to_string();
        let command = match session_notification.update.clone() {
            SessionUpdate::UserMessageChunk(chunk) => {
                self.set_prompt_id(session_id.clone(), uuid::Uuid::new_v4().to_string())
                    .await;
                parse::communication(chunk.content)
                    .map_err(Error::into_internal_error)
                    .and_then(|s| {
                        Commands::try_from(format!("User{}Message", s))
                            .map_err(Error::into_internal_error)
                    })
            }
            SessionUpdate::AgentMessageChunk(chunk) => parse::communication(chunk.content)
                .map_err(Error::into_internal_error)
                .and_then(|s| {
                    Commands::try_from(format!("Agent{}Message", s))
                        .map_err(Error::into_internal_error)
                }),
            SessionUpdate::AgentThoughtChunk(chunk) => parse::communication(chunk.content)
                .map_err(Error::into_internal_error)
                .and_then(|s| {
                    Commands::try_from(format!("Agent{}Thought", s))
                        .map_err(Error::into_internal_error)
                }),
            SessionUpdate::ToolCall(_) => Ok(Commands::ToolCall),
            SessionUpdate::ToolCallUpdate(_) => Ok(Commands::ToolCallUpdate),
            SessionUpdate::Plan(_) => Ok(Commands::Plan),
            SessionUpdate::AvailableCommandsUpdate(_) => Ok(Commands::AvailableCommands),
            SessionUpdate::CurrentModeUpdate(_) => Ok(Commands::ModeCurrent),
            SessionUpdate::ConfigOptionUpdate(_) => Ok(Commands::ConfigurationOption),
            SessionUpdate::UsageUpdate(_) => Ok(Commands::UsageUpdate),
            SessionUpdate::SessionInfoUpdate(_) => Ok(Commands::SessionUpdate),
            _ => return Err(Error::method_not_found()),
        }?;

        let hermes_notification = HermesNotification {
            session_id: session_notification.session_id.clone(),
            prompt_id: self.get_prompt_id(&session_id).await?,
            update: session_notification.update.clone(),
        };

        Ok((command, hermes_notification))
    }

    pub async fn session_notification(
        &self,
        session_notification: SessionNotification,
    ) -> Result<()> {
        if !self.can_receive_notifications().await {
            return Err(Error::method_not_found());
        }

        let session_id = session_notification.session_id.to_string();
        let (command, hermes_notification) =
            self.process_notification(&session_notification).await?;

        let state = self.state.lock().await;
        if state
            .agent_info
            .needs_local_history(state.config.session.store_history)
        {
            let key = format!("{}/{}.jsonl", state.agent_info.current, session_id);
            if let Ok(content) = serde_json::to_string(&session_notification) {
                state.agent_info.history.write_keyed(key, content);
            }
        }
        drop(state);

        Ok(self
            .execute_autocommand(command, hermes_notification)
            .await?)
    }

    /// Replay a session notification without writing to history.
    ///
    /// Intended for use during `load_session` when resuming from local history.
    /// Checks `can_receive_notifications()` once and logs a single warning if
    /// notifications are disabled, then replays all queued events.
    pub async fn replay_session_notifications(
        &self,
        session_notifications: Vec<SessionNotification>,
    ) -> acp::Result<()> {
        if self.can_receive_notifications().await {
            for notification in session_notifications {
                let (command, hermes_notification) =
                    self.process_notification(&notification).await?;
                self.execute_autocommand(command, hermes_notification)
                    .await?
            }
            Ok(())
        } else {
            Err(acp::error::Error::Permissions(
                "Notifications are disabled. Could not replay session history".to_string(),
            ))
        }
    }

    pub async fn write_text_file(
        &self,
        args: WriteTextFileRequest,
    ) -> Result<WriteTextFileResponse> {
        if !self.can_write().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = bounded::<WriteTextFileResponse>(1);
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::WriteTextFile,
            args.clone(),
            Responder::WriteFileResponse(sender, args),
        )
        .await?;
        receiver.recv().await.map_err(|e| {
            error!("{:?}", e);
            Error::internal_error()
        })
    }

    pub async fn read_text_file(&self, args: ReadTextFileRequest) -> Result<ReadTextFileResponse> {
        if !self.can_read().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = bounded::<Result<ReadTextFileResponse>>(1);
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::ReadTextFile,
            args.clone(),
            Responder::ReadFileResponse(sender, args),
        )
        .await?;
        receiver.recv().await.map_err(|e| {
            error!("{:?}", e);
            Error::internal_error()
        })?
    }

    pub async fn create_terminal(
        &self,
        args: CreateTerminalRequest,
    ) -> Result<CreateTerminalResponse> {
        if !self.can_access_terminal().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = bounded::<acp::Result<CreateTerminalResponse>>(1);
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::TerminalCreate,
            args.clone(),
            Responder::TerminalCreate(sender, args),
        )
        .await?;
        receiver
            .recv()
            .await
            .map_err(|e| {
                error!("{:?}", e);
                Error::internal_error()
            })?
            .map_err(|_e| Error::internal_error())
    }

    /// Gets the terminal output and exit status
    pub async fn terminal_output(
        &self,
        args: TerminalOutputRequest,
    ) -> Result<TerminalOutputResponse> {
        if !self.can_access_terminal().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = bounded::<acp::Result<TerminalOutputResponse>>(1);
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::TerminalOutput,
            args.clone(),
            Responder::TerminalOutput(sender, args),
        )
        .await?;
        receiver
            .recv()
            .await
            .map_err(|e| {
                error!("{:?}", e);
                Error::internal_error()
            })?
            .map_err(|_| Error::internal_error())
    }

    /// Waits for a terminal command to exit
    pub async fn wait_for_terminal_exit(
        &self,
        args: WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse> {
        if !self.can_access_terminal().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = bounded::<acp::Result<(Option<u32>, Option<String>)>>(1);
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::TerminalExit,
            args.clone(),
            Responder::TerminalExit(sender, args),
        )
        .await?;
        Ok(receiver
            .recv()
            .await
            .map_err(|_| Error::internal_error())?
            .and_then(|(exit_code, signal)| {
                // Validate that at least one field is present
                if exit_code.is_none() && signal.is_none() {
                    Err(acp::error::Error::InvalidInput(
                        "Both exit code and signal are undefined".to_string(),
                    ))
                } else {
                    let mut status = TerminalExitStatus::new();
                    if let Some(code) = exit_code {
                        status = status.exit_code(code);
                    }
                    if let Some(sig) = signal {
                        status = status.signal(sig);
                    }

                    Ok(WaitForTerminalExitResponse::new(status))
                }
            })?)
    }

    /// Releases a terminal resource
    pub async fn release_terminal(
        &self,
        args: ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse> {
        if !self.can_access_terminal().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = bounded::<acp::Result<ReleaseTerminalResponse>>(1);
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::TerminalRelease,
            args.clone(),
            Responder::TerminalRelease(sender, args),
        )
        .await?;
        receiver
            .recv()
            .await
            .map_err(|e| {
                error!("{:?}", e);
                Error::internal_error()
            })?
            .map_err(|_| Error::internal_error())
    }
}

fn session_id_from_scope(scope: &ElicitationScope) -> String {
    match scope {
        ElicitationScope::Session(s) => s.session_id.to_string(),
        _ => String::new(),
    }
}
