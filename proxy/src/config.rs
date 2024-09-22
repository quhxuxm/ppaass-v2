use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    worker_threads: usize,
    port: u16,
}

impl Configuration {
    pub fn worker_threads(&self) -> usize {
        self.worker_threads
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
