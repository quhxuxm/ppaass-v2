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
    dst_buffer_size: usize,
    #[access(get(ty(&std::path::Path)))]
    rsa_dir: PathBuf,
    #[access(get(ty(&str)))]
    max_log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 80,
            worker_threads: 256,
            dst_read_timeout: 120000,
            dst_write_timeout: 120000,
            dst_buffer_size: 1024 * 1024 * 8,
            max_log_level: "INFO".to_string(),
            rsa_dir: PathBuf::from("/resources/rsa"),
        }
    }
}
