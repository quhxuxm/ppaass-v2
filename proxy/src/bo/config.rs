use accessory::Accessors;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Debug, Clone, Serialize, Deserialize, Accessors)]
pub struct Config {
    #[access(get)]
    port: u16,
    #[access(get)]
    worker_threads: usize,
    #[access(get)]
    dst_read_timeout: u64,
    #[access(get)]
    dst_write_timeout: u64,
    #[access(get)]
    agent_read_timeout: u64,
    #[access(get)]
    agent_write_timeout: u64,
    #[access(get(ty(&std::path::Path)))]
    rsa_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 80,
            worker_threads: 256,
            dst_read_timeout: 120000,
            dst_write_timeout: 120000,
            agent_read_timeout: 120000,
            agent_write_timeout: 120000,
            rsa_dir: PathBuf::from("/resources/rsa"),
        }
    }
}
