use std::{sync::{Arc, Mutex, mpsc}, time::Duration};

use nvim_oxi::{api::{self, opts::CreateAutocmdOpts, types::AutocmdCallbackArgs}, serde::Deserializer};
use serde::de::DeserializeOwned;

pub fn wait_for_user_event<T>(
    pattern: &str,
    timeout: Duration,
) -> Box<dyn Fn() -> Result<T, nvim_oxi::Error>>
where
    T: DeserializeOwned + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<T>();

    // Wrap sender so closure remains `Fn`
    let tx = Arc::new(Mutex::new(Some(tx)));
    let tx_clone = tx.clone();

    let opts = CreateAutocmdOpts::builder()
        .patterns(vec![pattern])
        .callback(move |v: AutocmdCallbackArgs| {
            let mut deserializer = Deserializer::new(v.data);

            let parsed = T::deserialize(deserializer)
                .expect("Failed to deserialize autocmd data");

            if let Some(sender) = tx_clone.lock().unwrap().take() {
                let _ = sender.send(parsed);
            }

            false
        })
        .build();

    api::create_autocmd(vec!["User"], &opts).unwrap();

    Box::new(move || rx.recv_timeout(timeout)
        .map_err(|_| nvim_oxi::Error::Api))
}
