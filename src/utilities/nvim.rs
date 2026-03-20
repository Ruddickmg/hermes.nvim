use crate::acp::{Result, error::Error};
use nvim_oxi::IntoResult;
use nvim_oxi::libuv::AsyncHandle;
use std::sync::Arc;

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
        let handle = AsyncHandle::new(move || {
            while let Ok(data) = receiver.try_recv() {
                if let Err(err) = callback(data).into_result() {
                    eprintln!("Error in NvimHandler callback: {}", err);
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
