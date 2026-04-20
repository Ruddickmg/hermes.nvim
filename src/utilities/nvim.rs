use crate::acp::{Result, error::Error};
use crate::utilities::NvimRuntime;
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
    pub fn initialize<F, R, Fut>(nvim_runtime: NvimRuntime, mut callback: F) -> Result<Self>
    where
        F: FnMut(T) -> Fut + 'static,
        Fut: Future<Output = R>,
        R: IntoResult<()>,
        R::Error: std::error::Error + 'static,
    {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<T>(100);
        let handle = AsyncHandle::new(move || {
            while let Ok(data) = receiver.try_recv() {
                // CRITICAL: This callback is invoked from C code via FFI.
                // ANY panic that crosses this boundary will abort the process.
                // We use catch_unwind per-item so a panic on one item does not
                // prevent remaining queued items from being processed.
                // Note: We do NOT attempt to log panics here - if the logging
                // infrastructure is broken, we can't log. Silently swallow instead.
                let nvim_rt = nvim_runtime.clone();
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    nvim_rt.block_on_primary(async {
                        if let Err(err) = callback(data).await.into_result() {
                            error!("Error in NvimMessenger callback: {}", err);
                        }
                    });
                }))
                .ok();
            }
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
impl<T: Send + 'static> TransmitToNvim<T> for NvimMessenger<T> {
    fn blocking_send(&self, data: T) -> Result<()> {
        // Spawn a new thread with a tokio runtime to handle the blocking send
        // This avoids requiring a tokio runtime on the current thread
        let sender = self.sender.clone();
        let handle = self.handle.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                sender
                    .send(data)
                    .await
                    .map_err(|e| Error::Internal(e.to_string()))
            })?;
            handle.send().map_err(|e| Error::Internal(e.to_string()))
        })
        .join()
        .map_err(|_| Error::Internal("Thread panicked".to_string()))?
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
