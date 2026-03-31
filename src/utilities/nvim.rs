use crate::acp::{Result, error::Error};
use nvim_oxi::IntoResult;
use nvim_oxi::libuv::AsyncHandle;
use std::sync::Arc;
use tracing::error;

#[derive(Clone)]
pub struct NvimMessenger<T: 'static> {
    handle: Arc<AsyncHandle>,
    sender: Arc<tokio::sync::mpsc::Sender<T>>,
}

impl<T> NvimMessenger<T> {
    pub fn initialize<F, R>(mut callback: F) -> Result<Self>
    where
        F: FnMut(T) -> R + 'static,
        R: IntoResult<()>,
        R::Error: std::error::Error + 'static,
    {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<T>(100);
        let handle =
            AsyncHandle::new(move || {
                // CRITICAL: This callback is invoked from C code via FFI.
                // ANY panic that crosses this boundary will abort the process.
                // We use catch_unwind to prevent this.
                // Note: We do NOT attempt to log panics here - if the logging
                // infrastructure is broken, we can't log. Silently swallow instead.
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    while let Ok(data) = receiver.try_recv() {
                        if let Err(err) = callback(data).into_result() {
                            error!("Error in NvimHandler callback: {}", err);
                        }
                    }
                })).ok();
            })
            .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(Self {
            handle: Arc::new(handle),
            sender: Arc::new(sender),
        })
    }
}

#[async_trait::async_trait(?Send)]
pub trait TransmitToNvim<T> {
    fn blocking_send(&self, data: T) -> Result<()>;
    async fn send(&self, data: T) -> Result<()>;
}

#[async_trait::async_trait(?Send)]
impl<T> TransmitToNvim<T> for NvimMessenger<T> {
    fn blocking_send(&self, data: T) -> Result<()> {
        self.sender
            .blocking_send(data)
            .map_err(|e| Error::Internal(e.to_string()))?;
        self.handle
            .send()
            .map_err(|e| Error::Internal(e.to_string()))
    }

    async fn send(&self, data: T) -> Result<()> {
        self.sender
            .send(data)
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;
        self.handle
            .send()
            .map_err(|e| Error::Internal(e.to_string()))
    }
}
