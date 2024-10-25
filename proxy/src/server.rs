use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use crate::config::Config;
use crate::error::{ ServerError};
pub struct Server{
    config: Arc<Config>,
}

impl Server{
    pub fn new(config: Arc<Config>) -> Self{
        Self{
            config
        }
    }

    pub async fn start(&self)->Result<(), ServerError>{
        let runtime = Builder::new_multi_thread().worker_threads(*self.config.worker_threads()).enable_all().build()?;
        let config = self.config.clone();
        runtime.block_on(async move {
            let tcp_listener = match TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0,0,0,0)), *config.port())).await{
                Ok(_) => {}
                Err(_) => {}
            };
        });
        Ok(())
    }

}