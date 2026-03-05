use hermes::{apc, nvim::autocommands::Commands};
use nvim_oxi::{
    api::{self, opts::CreateAutocmdOpts, types::AutocmdCallbackArgs},
    serde::Deserializer,
};
use serde::de::DeserializeOwned;
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub fn listen_for_autocommand<T>(
    pattern: Commands,
) -> Box<dyn Fn(Duration) -> Result<T, nvim_oxi::Error>>
where
    T: Debug + DeserializeOwned + Send + Clone + 'static,
{
    let pattern_string = pattern.to_string();
    let result: Arc<Mutex<Option<T>>> = Arc::new(Mutex::new(None));
    let copy = result.clone();

    let opts = CreateAutocmdOpts::builder()
        .group("hermes")
        .patterns(vec![pattern_string.as_str()])
        .callback(move |v: AutocmdCallbackArgs| {
            println!("Received autocmd callback");

            let parsed: T = T::deserialize(Deserializer::new(v.data))
                .expect("Failed to deserialize autocmd data");

            *result.lock().unwrap() = Some(parsed);

            false
        })
        .build();

    api::create_autocmd(vec!["User"], &opts).unwrap();

    Box::new(move |duration| {
        let start = Instant::now();

        loop {
            if let Some(value) = copy.lock().unwrap().clone() {
                return Ok(value);
            }

            if start.elapsed() > duration {
                return Err(nvim_oxi::Error::Api(
                    api::Error::Other("Timed out waiting for Autocmd".into()),
                ));
            }

            // tick Neovim's input loop
            api::input("<Ignore>")?;
        }
    })
}
