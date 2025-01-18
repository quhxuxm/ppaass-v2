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
    dst_read_timeout: Option<u64>,
    #[access(get)]
    dst_write_timeout: Option<u64>,
    #[access(get)]
    dst_connect_timeout: u64,
    #[access(get)]
    dst_tcp_keepalive_interval: u64,
    #[access(get)]
    dst_tcp_keepalive_time: u64,
    #[access(get)]
    dst_tcp_keepalive_retry: u32,
    #[access(get)]
    dst_buffer_size: usize,
    #[access(get)]
    dst_socket_send_buffer_size: Option<usize>,
    #[access(get)]
    dst_socket_receive_buffer_size: Option<usize>,
    #[access(get)]
    agent_buffer_size: usize,
    #[access(get(ty(&std::path::Path)))]
    rsa_dir: PathBuf,
    #[access(get(ty(&std::path::Path)))]
    forward_rsa_dir: PathBuf,
    #[access(get(ty(&str)))]
    max_log_level: String,
    #[access(get)]
    agent_connection_tcp_keepalive: bool,
    #[access(get)]
    agent_connection_tcp_keepalive_interval: u64,
    #[access(get)]
    agent_connection_tcp_keepalive_time: u64,
    #[access(get)]
    agent_connection_tcp_keepalive_retry: u32,
    #[access(get)]
    agent_socket_send_buffer_size: Option<usize>,
    #[access(get)]
    agent_socket_receive_buffer_size: Option<usize>,
    #[access(get)]
    agent_connection_write_timeout: Option<u64>,
    #[access(get)]
    agent_connection_read_timeout: Option<u64>,
    #[access(get)]
    server_socket_backlog: u16,
    #[access(get)]
    forward_server_addresses: Option<Vec<String>>,
    #[access(get)]
    forward_auth_token: Option<String>,
    #[access(get)]
    log_folder: PathBuf,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            port: 80,
            worker_threads: 256,
            dst_read_timeout: None,
            dst_write_timeout: None,
            dst_buffer_size: 1024 * 1024 * 8,
            dst_socket_send_buffer_size: None,
            dst_socket_receive_buffer_size: None,
            agent_buffer_size: 1024 * 1024 * 8,
            max_log_level: "INFO".to_string(),
            rsa_dir: PathBuf::from("/resources/rsa"),
            forward_rsa_dir: PathBuf::from("/resources/forward_rsa"),
            agent_connection_write_timeout: None,
            agent_connection_read_timeout: None,
            agent_connection_tcp_keepalive: false,
            agent_connection_tcp_keepalive_interval: 75,
            agent_connection_tcp_keepalive_time: 7200,
            agent_connection_tcp_keepalive_retry: 9,
            agent_socket_send_buffer_size: None,
            agent_socket_receive_buffer_size: None,
            server_socket_backlog: 1024,
            dst_connect_timeout: 20,
            dst_tcp_keepalive_interval: 75,
            dst_tcp_keepalive_time: 7200,
            dst_tcp_keepalive_retry: 9,
            forward_server_addresses: Some(vec!["127.0.0.1".to_string()]),
            forward_auth_token: None,
            log_folder: PathBuf::from("/logs"),
        }
    }
}
