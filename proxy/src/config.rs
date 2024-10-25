use accessory::Accessors;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, Accessors, Default)]
pub(crate) struct Config{
    #[access(get)]
    #[default(80)]
    port: u16,
    #[access(get)]
    #[default(256)]
    worker_threads: usize,
    #[access(get)]
    #[default(120000)]
    dst_read_timeout: u64,
    #[access(get)]
    #[default(120000)]
    dst_write_timeout: u64,
    #[access(get)]
    #[default(120000)]
    agent_read_timeout: u64,
    #[access(get)]
    #[default(120000)]
    agent_write_timeout: u64,
}