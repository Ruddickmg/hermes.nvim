use crate::acp::{Result, error::Error};
use crate::utilities::NvimRuntime;
use nvim_oxi::IntoResult;
use nvim_oxi::libuv::AsyncHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use async_channel::{Sender, Receiver};
use futures_lite::future::yield_now;
use tracing::error;

#[derive(Clone)]
pub struct NvimMessenger<T: 'static> {
    handle: Arc<AsyncHandle>,
    sender: Arc<Sender<T>>,
}

impl<T> NvimMessenger<T> {
    pub fn initialize<F, R, Fut>(nvim_runtime: NvimRuntime, callback: F) -> Result<Self>
    where
        F: FnMut(T) -> Fut + 'static,
        Fut: Future<Output = R>,
        R: IntoResult<()>,
        R::Error: std::error::Error + 'static,
    {
        let (sender, receiver) = async_channel::bounded::<T>(100);
        let callback = Rc::new(RefCell::new(callback));
        let handle = AsyncHandle::new(move || {
            // Process all available messages from the channel
            // We use try_recv in a loop to drain the channel
            loop {
                match receiver.try_recv() {
                    Ok(data) => {
                        let nvim_rt = nvim_runtime.clone();
                        let cb = callback.clone();
                        // CRITICAL: AsyncHandle callbacks are invoked by libuv during
                        // uv_run(), which may be in the middle of processing other
                        // Neovim events (e.g., BufNewFile autocommands during startup).
                        // Calling Neovim APIs directly here causes re-entrant event
                        // processing at the C level, corrupting internal state and
                        // crashing Neovim. We use nvim_oxi::schedule (vim.schedule)
                        // to defer all Neovim API work to a safe point in the event
                        // loop where Neovim's state is consistent.
                        nvim_oxi::schedule(move |_| {
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                nvim_rt.block_on_primary(async {
                                    if let Err(err) = cb.borrow_mut()(data).await.into_result() {
                                        error!("Error in NvimMessenger callback: {}", err);
                                    }
                                });
                            }))
                            .ok();
                            Ok::<_, nvim_oxi::Error>(())
                        });
                    }
                    Err(async_channel::TryRecvError::Empty) => {
                        // No more messages available
                        break;
                    }
                    Err(async_channel::TryRecvError::Closed) => {
                        // Channel closed, stop processing
                        break;
                    }
                }
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
    async fn send(&self, data: T) -> Result<()>;
}

#[async_trait::async_trait(?Send)]
impl<T: Send + 'static> TransmitToNvim<T> for NvimMessenger<T> {
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
