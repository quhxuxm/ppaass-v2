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
    server_socket_backlog: u16,
    #[access(get)]
    client_connection_tcp_keepalive: bool,
    #[access(get)]
    client_connection_tcp_keepalive_interval: u64,
    #[access(get)]
    client_connection_tcp_keepalive_time: u64,
    #[access(get)]
    client_connection_tcp_keepalive_retry: u32,
    #[access(get)]
    client_connection_read_timeout: u64,
    #[access(get)]
    client_connection_write_timeout: u64,
    #[access(get)]
    client_relay_buffer_size: usize,
    #[access(get)]
    proxy_relay_buffer_size: usize,
    #[access(get)]
    proxy_connection_pool_size: Option<usize>,
    #[access(get)]
    proxy_connection_start_check_timer: bool,
    #[access(get)]
    proxy_connection_start_check_timer_interval: u64,
    #[access(get)]
    proxy_connection_max_lifetime: i64,
    #[access(get)]
    proxy_connection_check_interval: i64,
    #[access(get)]
    proxy_connection_pool_fill_interval: Option<u64>,
    #[access(get)]
    proxy_connect_timeout: u64,
    #[access(get)]
    proxy_connection_read_timeout: u64,
    #[access(get)]
    proxy_connection_write_timeout: u64,
    #[access(get)]
    proxy_connection_tcp_keepalive_interval: u64,
    #[access(get)]
    proxy_connection_tcp_keepalive_time: u64,
    #[access(get)]
    proxy_connection_tcp_keepalive_retry: u32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            port: 80,
            auth_token: "user1".to_string(),
            proxy_addresses: vec!["45.76.0.10:80".to_string()],
            worker_threads: 256,
            max_log_level: "INFO".to_string(),
            rsa_dir: PathBuf::from("/resources/agent/rsa"),
            client_connection_tcp_keepalive: false,
            client_connection_tcp_keepalive_interval: 120,
            client_connection_tcp_keepalive_time: 5,
            client_connection_tcp_keepalive_retry: 3,
            server_socket_backlog: 1024,
            client_relay_buffer_size: 1024 * 1024 * 8,
            proxy_relay_buffer_size: 1024 * 1024 * 8,
            proxy_connection_pool_size: Some(32),
            proxy_connection_start_check_timer: false,
            proxy_connection_check_interval: 60,
            proxy_connection_pool_fill_interval: Some(20),
            proxy_connect_timeout: 20,
            proxy_connection_read_timeout: 20,
            proxy_connection_write_timeout: 20,
            proxy_connection_tcp_keepalive_interval: 120,
            proxy_connection_tcp_keepalive_time: 5,
            proxy_connection_tcp_keepalive_retry: 3,
            client_connection_read_timeout: 20,
            client_connection_write_timeout: 20,
            proxy_connection_start_check_timer_interval: 120,
            proxy_connection_max_lifetime: 300,
        }
    }
}
