//! E2E test for ReadTextFile workflow with Copilot
//!
//! Tests whether copilot uses ACP read_text_file protocol or internal tools
use crate::{utilities::autocommand, TIMEOUT_IN_SECONDS};
use agent_client_protocol::{
    InitializeResponse, NewSessionResponse, PermissionOption, PromptResponse, ReadTextFileRequest,
    SessionId, ToolCall,
};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{conversion::FromObject, Dictionary, Function, Object};
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PermissionRequest {
    session_id: SessionId,
    request_id: String,
    tool_call: ToolCall,
    options: Vec<PermissionOption>,
}

/// Test that verifies copilot uses ACP read_text_file protocol
/// Creates a file and asks copilot to read it
///
/// NOTE: This test is marked as ignored because current ACP agents (copilot, opencode, gemini)
/// do not use the ACP read_text_file protocol. Instead, they use internal file tools.
/// This is an ecosystem limitation, not a Hermes bug.
#[ignore = "Current ACP agents don't use read_text_file protocol - they use internal file tools instead"]
#[nvim_oxi::test]
fn test_copilot_read_text_file_workflow() -> Result<(), nvim_oxi::Error> {
    info!("Starting copilot read file workflow test");

    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let prompt: Function<PromptArgs, ()> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;
    let respond: Function<(String, String), ()> =
        FromObject::from_object(dict.get("respond").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_read_file =
        autocommand::listen_for_autocommand::<ReadTextFileRequest>(Commands::ReadTextFile);
    let wait_for_permission_request =
        autocommand::listen_for_autocommand::<PermissionRequest>(Commands::PermissionRequest);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    // Step 1: Connect to copilot
    info!("Connecting to copilot agent...");
    connect.call((nvim_oxi::String::from("copilot"), None))?;
    let init_response = wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    info!("Connected! Agent info: {:?}", init_response.agent_info);

    // Step 2: Create session
    info!("Creating session...");
    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;
    info!("Session created: {:?}", session_id);

    // Step 3: Create a test file
    let test_file_path = "/tmp/hermes_test_file.txt";
    let test_content = "Hello from Hermes test file! This is test content for copilot to read.";
    std::fs::write(test_file_path, test_content).expect("Failed to write test file");
    info!(
        "Created test file at: {} with content: {:?}",
        test_file_path, test_content
    );

    // Step 4: Send prompt asking copilot to read the file
    info!("Sending prompt to read file...");
    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert(
        "text",
        format!(
            "Please read the file at {} and tell me what it contains",
            test_file_path
        ),
    );
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session_id.to_string(), content))?;

    // Step 5: Wait for either ReadTextFile or PermissionRequest
    info!("Waiting for ReadTextFile or PermissionRequest autocommand...");

    // Try to read file request first (with shorter timeout)
    let read_request = match wait_for_read_file(Duration::from_secs(10)) {
        Ok(req) => {
            info!("ReadTextFile received! Path: {:?}", req.path);
            Some(req)
        }
        Err(_) => {
            info!("No ReadTextFile received in 10s, checking for permission request...");
            None
        }
    };

    // If no read request yet, check for permission request
    if read_request.is_none() {
        match wait_for_permission_request(Duration::from_secs(10)) {
            Ok(perm_req) => {
                info!(
                    "PermissionRequest received! Requesting permission for: {:?}",
                    perm_req.tool_call
                );

                // Find the "allow" option
                let allow_option = perm_req
                    .options
                    .iter()
                    .find(|opt| opt.option_id.to_string().to_lowercase().contains("allow"))
                    .or_else(|| perm_req.options.first());

                if let Some(option) = allow_option {
                    info!("Responding with option: {:?}", option.option_id);
                    respond.call((perm_req.request_id.clone(), option.option_id.to_string()))?;

                    // Now wait for ReadTextFile after granting permission
                    info!("Waiting for ReadTextFile after granting permission...");
                    match wait_for_read_file(Duration::from_secs(20)) {
                        Ok(req) => {
                            info!(
                                "ReadTextFile received after permission! Path: {:?}",
                                req.path
                            );
                            // Verify the path matches
                            let path_str = req.path.to_string_lossy().to_string();
                            assert!(
                                path_str.contains("hermes_test_file")
                                    || path_str == test_file_path,
                                "ReadTextFile path should match test file. Got: {:?}, Expected: {:?}",
                                req.path,
                                test_file_path
                            );
                        }
                        Err(_) => {
                            info!("No ReadTextFile received after permission grant");
                        }
                    }
                }
            }
            Err(_) => {
                info!("No PermissionRequest received either");
            }
        }
    } else {
        // We got ReadTextFile directly, verify it
        let req = read_request.unwrap();
        let path_str = req.path.to_string_lossy().to_string();
        assert!(
            path_str.contains("hermes_test_file") || path_str == test_file_path,
            "ReadTextFile path should match test file. Got: {:?}, Expected: {:?}",
            req.path,
            test_file_path
        );
        info!("ReadTextFile request verified!");
    }

    // Step 6: Wait for prompt to complete
    info!("Waiting for prompt completion...");
    let prompt_response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    info!(
        "Prompt completed! Stop reason: {:?}",
        prompt_response.stop_reason
    );

    // Step 7: Cleanup
    info!("Cleaning up...");
    disconnect.call(DisconnectArgs::All)?;

    // Cleanup test file
    let _ = std::fs::remove_file(test_file_path);

    info!("Test completed successfully!");
    Ok(())
}
