use hermes::apc;
use hermes::nvim::autocommands::Commands;
use nvim_oxi::api::{self, opts::CreateAutocmdOpts, types::AutocmdCallbackArgs};
use nvim_oxi::conversion::FromObject;
use nvim_oxi::{Object, serde::Deserializer};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::sync::mpsc;
use std::{
    fmt::Debug,
    rc::Rc,
    time::{Duration, Instant},
};

fn object_to_struct<T>(obj: Object) -> Result<T, Box<dyn std::error::Error>>
where
    T: for<'de> Deserialize<'de>,
{
    let json_str = api::json_encode(&obj)?;

    // Step 2: Deserialize JSON string → struct
    let result: T = serde_json::from_str(&json_str)?;

    Ok(result)
}

pub fn listen_for_autocommand<T>(
    pattern: Commands,
) -> Box<dyn Fn(Duration) -> Result<T, nvim_oxi::Error>>
where
    T: Debug + DeserializeOwned + Send + Clone + 'static,
{
    let pattern_string = pattern.to_string();
    let (tx, reciever) = mpsc::channel::<T>();
    let sender = Rc::new(tx);

    let opts = CreateAutocmdOpts::builder()
        .group("hermes")
        .patterns(vec![pattern_string.as_str()])
        .callback(move |v: AutocmdCallbackArgs| {
            println!("Received autocmd callback");

            let parsed: T = from_object(v.data)
                .map_err(|e| {
                    println!("Failed to deserialize autocmd data: {:?}", e);
                    e
                })
                .expect("Failed to deserialize autocmd data");

            println!("Parsed autocmd data: {:?}", parsed);
            sender.send(parsed).unwrap();

            false
        })
        .build();

    api::create_autocmd(vec!["User"], &opts).unwrap();

    Box::new(move |duration| {
        let start = Instant::now();
        loop {
            println!("Waiting for autocmd response...");
            if let Ok(response) = reciever.try_recv() {
                println!("Received response: {:?}", response);
                return Ok(response);
            }
            if start.elapsed() > duration {
                println!("Timed out waiting for autocmd response");
                return Err(nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(
                    "Timed out waiting for Autocmd".into(),
                )));
            }
            nvim_oxi::api::command("sleep 100m")?;
        }
    })
}

// fn test_initialization() -> Result<(), nvim_oxi::Error> {
//     let dict: Dictionary = hermes()?;
//     let connect: Function<Option<ConnectionArgs>, ()> =
//         FromObject::from_object(dict.get("connect").unwrap().clone())?;
//     let disconnect: Function<DisconnectArgs, ()> =
//         FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
//
//     let (tx, rx) = std::sync::mpsc::channel::<InitializeResponse>();
//     let pattern_string = Commands::AgentConnectionInitialized.to_string();
//     let tx_cell = std::cell::RefCell::new(Some(tx));
//
//     let opts = nvim_oxi::api::opts::CreateAutocmdOpts::builder()
//         .group("hermes")
//         .patterns(vec![pattern_string.as_str()])
//         .callback(move |v: nvim_oxi::api::types::AutocmdCallbackArgs| {
//             if let Ok(parsed) = InitializeResponse::deserialize(Deserializer::new(v.data)) {
//                 if let Some(tx) = tx_cell.borrow_mut().take() {
//                     let _ = tx.send(parsed);
//                 }
//             }
//             false
//         })
//         .build();
//
//     nvim_oxi::api::create_autocmd(vec!["User"], &opts).unwrap();
//
//     connect.call(Some(ConnectionArgs {
//         agent: Some(Assistant::Opencode),
//         protocol: Some(Protocol::Stdio),
//     }))?;
//
//     let start = std::time::Instant::now();
//     let timeout = Duration::from_secs(10);
//
//     let response = loop {
//         if let Ok(response) = rx.try_recv() {
//             break response;
//         }
//
//         if start.elapsed() > timeout {
//             return Err(nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(
//                 "Timed out waiting for Autocmd".into(),
//             )));
//         }
//
//         nvim_oxi::api::command("sleep 100m")?;
//     };
//
//     assert_eq!(response.agent_info.as_ref().unwrap().name, "OpenCode");
//
//     disconnect.call(DisconnectArgs::All)?;
//
//     Ok(())
// }
