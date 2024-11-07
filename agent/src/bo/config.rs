use accessory::Accessors;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Debug, Clone, Serialize, Deserialize, Accessors)]
pub struct Config {
    #[access(get)]
    port: u16,
    #[access(get(ty(&str)))]
    auth_token: String,
    #[access(get)]
    worker_threads: usize,
    #[access(get)]
    proxy_read_timeout: u64,
    #[access(get)]
    proxy_write_timeout: u64,
    #[access(get)]
    client_read_timeout: u64,
    #[access(get)]
    client_write_timeout: u64,
    #[access(get(ty(&std::path::Path)))]
    rsa_dir: PathBuf,
}
