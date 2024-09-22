use crate::error::ServerError;
use bytes::{Bytes, BytesMut};
use std::sync::Mutex;
pub struct Buffer {
    buffer: Mutex<BytesMut>,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            buffer: Mutex::new(BytesMut::with_capacity(65536)),
        }
    }

    pub fn receive(&self, data: &[u8]) -> Result<(), ServerError> {
        let mut buffer_lock = self
            .buffer
            .lock()
            .map_err(|e| ServerError::Lock(format!("{e:?}")))?;
        buffer_lock.extend(data);
        Ok(())
    }

    pub fn consume(&self) -> Result<Bytes, ServerError> {
        let mut buffer_lock = self
            .buffer
            .lock()
            .map_err(|e| ServerError::Lock(format!("{e:?}")))?;
        Ok(buffer_lock.split().freeze())
    }
}
