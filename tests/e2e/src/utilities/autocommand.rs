use hermes::nvim::autocommands::Commands;
use nvim_oxi::api::{self, opts::CreateAutocmdOpts, types::AutocmdCallbackArgs};
use nvim_oxi::{Object, serde::Deserializer};
use serde::de::DeserializeOwned;
use std::sync::mpsc;
use std::{
    fmt::Debug,
    rc::Rc,
    time::{Duration, Instant},
};
use tracing::error;

pub fn nvim_object_to_struct<T>(obj: Object) -> Result<T, nvim_oxi::Error>
where
    T: DeserializeOwned,
{
    T::deserialize(Deserializer::new(obj))
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))
}

pub fn listen_for_autocommand<T>(
    autocommand: Commands,
) -> Box<dyn Fn(Duration) -> Result<T, nvim_oxi::Error>>
where
    T: Debug + DeserializeOwned + Send + Clone + 'static,
{
    let pattern = autocommand.to_string();
    let (tx, receiver) = mpsc::channel::<T>();
    let sender = Rc::new(tx);
    let opts = CreateAutocmdOpts::builder()
        .group("hermes")
        .patterns(vec![pattern.as_str()])
        .callback(move |v: AutocmdCallbackArgs| {
            match nvim_object_to_struct(v.data) {
                Ok(parsed) => sender.send(parsed).unwrap(),
                Err(e) => error!("Error occurred while parsing: {:#?}", e),
            };
            false
        })
        .build();

    api::create_autocmd(vec!["User"], &opts).unwrap();

    Box::new(move |duration| {
        let start = Instant::now();
        loop {
            if let Ok(response) = receiver.try_recv() {
                break Ok(response);
            }
            if start.elapsed() > duration {
                break Err(nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(
                    "Timed out waiting for Autocmd".into(),
                )));
            }
            nvim_oxi::api::command("sleep 100m")?;
        }
    })
}
