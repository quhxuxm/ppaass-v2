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
    proxy_addresses: Vec<String>,
    #[access(get)]
    worker_threads: usize,
    #[access(get(ty(&std::path::Path)))]
    rsa_dir: PathBuf,
    #[access(get(ty(&str)))]
    max_log_level: String,
    #[access(get)]
    client_relay_buffer_size: usize,
    #[access(get)]
    proxy_relay_buffer_size: usize,
    #[access(get)]
    proxy_socket_recv_buffer_size: u32,
    #[access(get)]
    proxy_socket_send_buffer_size: u32,
    #[access(get)]
    proxy_connection_pool_size: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            port: 80,
            auth_token: "user1".to_string(),
            proxy_addresses: vec!["45.32.10.177:80".to_string()],
            worker_threads: 256,
            max_log_level: "INFO".to_string(),
            rsa_dir: PathBuf::from("/resources/rsa"),
            client_relay_buffer_size: 1024 * 1024 * 8,
            proxy_relay_buffer_size: 1024 * 1024 * 8,
            proxy_connection_pool_size: 32,
            proxy_socket_recv_buffer_size: 1024 * 1024 * 8,
            proxy_socket_send_buffer_size: 1024 * 1024 * 8,
        }
    }
}
