//! HTTP transport: connect to a remote ACP agent via WebSocket and drive an
//! ACP connection over the socket.
//!
//! Like the `tcp` and `stdio` modules, this owns the per-protocol orchestration.
//! The ACP `Client.builder()` plumbing is shared via
//! [`crate::acp::connection::connect::handle_connection`].

use crate::{
    Handler,
    acp::{
        connection::{Assistant, Protocol, UserRequest, connect::handle_connection},
        error::Error,
    },
};
use agent_client_protocol::Lines;
use async_channel::Receiver;
use async_tungstenite::tungstenite::{Error as WsError, Message};
use futures::{Stream, StreamExt};
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// Maps a single WebSocket message into an `io::Result<String>`.
fn map_ws_message(msg: Result<Message, WsError>) -> std::io::Result<String> {
    match msg {
        Ok(Message::Text(text)) => Ok(text.to_string()),
        Ok(Message::Close(_)) => Err(std::io::Error::new(
            std::io::ErrorKind::ConnectionAborted,
            "WebSocket closed",
        )),
        Ok(other) => {
            debug!("Received non-text WebSocket message: {:?}", other);
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unexpected WebSocket message: {:?}", other),
            ))
        }
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}

/// Converts a `WebSocketReceiver` into a `Stream<Item = io::Result<String>>`
/// by extracting text messages and mapping errors.
fn ws_text_stream<S>(
    ws_receiver: async_tungstenite::WebSocketReceiver<S>,
) -> impl Stream<Item = std::io::Result<String>> + Send + 'static
where
    S: futures::AsyncRead + futures::AsyncWrite + Unpin + Send + 'static,
{
    ws_receiver.map(map_ws_message)
}

#[instrument(level = "trace", skip(client, request_receiver))]
pub async fn connect(
    protocol: Protocol,
    client: Arc<Handler>,
    agent: Assistant,
    request_receiver: Receiver<UserRequest>,
) -> Result<(), Error> {
    let url = build_url(protocol, &agent).inspect_err(|e| {
        error!("Failed to build WebSocket URL: {}", e);
    })?;

    debug!("Connecting to agent at {}", url);

    let (ws_stream, _) = async_tungstenite::smol::connect_async(&url)
        .await
        .map_err(|e| Error::Connection(format!("Failed to connect to {}: {}", url, e)))?;

    info!("Connected to agent '{}' via websocket at {}", agent, url);

    let (ws_sender, ws_receiver) = ws_stream.split();

    // Build a Sink<String, Error = io::Error> from the WebSocketSender
    let outgoing_sink = futures::sink::unfold(ws_sender, |mut sender, text: String| async move {
        sender
            .send(Message::Text(text.into()))
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok::<_, std::io::Error>(sender)
    });

    let incoming_stream = ws_text_stream(ws_receiver);

    let lines = Lines::new(outgoing_sink, incoming_stream);

    let result = handle_connection(client, agent.clone(), request_receiver, lines).await;

    info!("Disconnected from '{}' via websocket", agent);
    result
}

/// Build the WebSocket URL from protocol and assistant configuration.
fn build_url(protocol: Protocol, agent: &Assistant) -> Result<String, Error> {
    let scheme = match protocol {
        Protocol::Https => "wss",
        _ => "ws",
    };
    match agent {
        Assistant::CustomUrl {
            host, port, path, ..
        } => {
            let mut url = format!("{}://{}:{}", scheme, host, port);
            if let Some(path) = path {
                if path.starts_with('/') {
                    url.push_str(path);
                } else {
                    url.push('/');
                    url.push_str(path);
                }
            }
            Ok(url)
        }
        other => Err(Error::Connection(format!(
            "HTTP protocol requires a CustomUrl assistant, got {}",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn build_url_http_without_path() {
        let agent = Assistant::CustomUrl {
            name: String::from("test-agent"),
            host: String::from("localhost"),
            port: 8080,
            path: None,
        };
        let result = build_url(Protocol::Http, &agent).unwrap();
        assert_eq!(result, "ws://localhost:8080");
    }

    #[test]
    fn build_url_https_without_path() {
        let agent = Assistant::CustomUrl {
            name: String::from("test-agent"),
            host: String::from("api.example.com"),
            port: 443,
            path: None,
        };
        let result = build_url(Protocol::Https, &agent).unwrap();
        assert_eq!(result, "wss://api.example.com:443");
    }

    #[test]
    fn build_url_http_with_path() {
        let agent = Assistant::CustomUrl {
            name: String::from("test-agent"),
            host: String::from("localhost"),
            port: 8080,
            path: Some(String::from("/acp")),
        };
        let result = build_url(Protocol::Http, &agent).unwrap();
        assert_eq!(result, "ws://localhost:8080/acp");
    }

    #[test]
    fn build_url_https_with_path() {
        let agent = Assistant::CustomUrl {
            name: String::from("test-agent"),
            host: String::from("api.example.com"),
            port: 443,
            path: Some(String::from("/v1/acp")),
        };
        let result = build_url(Protocol::Https, &agent).unwrap();
        assert_eq!(result, "wss://api.example.com:443/v1/acp");
    }

    #[test]
    fn build_url_http_with_path_missing_leading_slash() {
        let agent = Assistant::CustomUrl {
            name: String::from("test-agent"),
            host: String::from("localhost"),
            port: 8080,
            path: Some(String::from("acp")),
        };
        let result = build_url(Protocol::Http, &agent).unwrap();
        assert_eq!(result, "ws://localhost:8080/acp");
    }

    #[test]
    fn build_url_rejects_non_custom_url() {
        let agent = Assistant::CustomStdio {
            name: String::from("copilot"),
            command: String::from("copilot"),
            args: vec![String::from("--acp")],
        };
        let result = build_url(Protocol::Http, &agent);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("HTTP protocol requires a CustomUrl assistant")
        );
    }

    #[test]
    fn map_ws_message_text_returns_ok() {
        let result = map_ws_message(Ok(Message::Text("hello".into())));
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn map_ws_message_close_returns_connection_aborted() {
        let result = map_ws_message(Ok(Message::Close(None)));
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::ConnectionAborted);
    }

    #[test]
    fn map_ws_message_binary_returns_invalid_data() {
        let result = map_ws_message(Ok(Message::Binary(vec![1u8, 2, 3].into())));
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn map_ws_message_error_returns_other() {
        let ws_err = WsError::from(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
        let result = map_ws_message(Err(ws_err));
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::Other);
    }
}
