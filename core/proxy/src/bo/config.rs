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
    dst_connect_timeout: u64,
    #[access(get)]
    dst_tcp_keepalive: u64,
    #[access(get)]
    dst_buffer_size: usize,
    #[access(get)]
    agent_buffer_size: usize,
    #[access(get(ty(&std::path::Path)))]
    rsa_dir: PathBuf,
    #[access(get(ty(&str)))]
    max_log_level: String,
    #[access(get)]
    agent_connection_tcp_keep_alive: u64,
    #[access(get)]
    agent_connection_write_timeout: u64,
    #[access(get)]
    agent_connection_read_timeout: u64,
    #[access(get)]
    server_socket_backlog: u16,

}
impl Default for Config {
    fn default() -> Self {
        Self {
            port: 80,
            worker_threads: 256,
            dst_read_timeout: 120000,
            dst_write_timeout: 120000,
            dst_buffer_size: 1024 * 1024 * 8,
            agent_buffer_size: 1024 * 1024 * 8,
            max_log_level: "INFO".to_string(),
            rsa_dir: PathBuf::from("/resources/rsa"),
            agent_connection_write_timeout: 20,
            agent_connection_read_timeout: 20,
            agent_connection_tcp_keep_alive: 120,
            server_socket_backlog: 1024,
            dst_connect_timeout: 20,
            dst_tcp_keepalive: 120,
        }
    }
}
