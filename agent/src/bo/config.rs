use accessory::Accessors;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Debug, Clone, Serialize, Deserialize, Accessors)]
pub struct Config {
    #[access(get)]
    port: u16,
    #[access(get(ty(&str)))]
    auth_token: String,
    #[access(get(ty(&str)))]
    proxy_create_session_entry: String,
    #[access(get(ty(&str)))]
    proxy_relay_entry: String,
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
    #[access(get(ty(&str)))]
    max_log_level: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            port: 80,
            auth_token: "user1".to_string(),
            proxy_create_session_entry: "http://localhost:8080/session/create".to_string(),
            proxy_relay_entry: "ws://localhost:8080/relay".to_string(),
            worker_threads: 256,
            proxy_read_timeout: 120000,
            proxy_write_timeout: 120000,
            client_read_timeout: 120000,
            client_write_timeout: 120000,
            max_log_level: "INFO".to_string(),
            rsa_dir: PathBuf::from("/resources/rsa"),
        }
    }
}
