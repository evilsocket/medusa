use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use log::{debug, warn};
use russh::server::{self, Config as SSHConfig};

use crate::{
    config::{Config as MainConfig, Service},
    protocols::{Error, Protocol},
};

use super::{
    config::{self, Config},
    handler,
};

#[derive(Clone)]
pub struct Server {
    service_name: String,
    service: Arc<Mutex<Service>>,
    config: Arc<Config>,
    ssh_config: Arc<SSHConfig>,
    main_config: Arc<MainConfig>,
}

impl Server {
    pub fn new(
        service_name: String,
        service: Arc<Mutex<Service>>,
        main_config: MainConfig,
    ) -> Result<Self, Error> {
        let config = config::from_service(service.lock().as_ref().unwrap());
        let ssh_config = config.to_ssh_config()?;
        let config = Arc::new(config);
        let ssh_config = Arc::new(ssh_config);
        let main_config = Arc::new(main_config);

        Ok(Server {
            service_name,
            service,
            config,
            ssh_config,
            main_config,
        })
    }
}

#[async_trait]
impl Protocol for Server {
    async fn run(&self) {
        debug!("starting ssh on {} ...", &self.config.address);

        let address = std::net::SocketAddr::from_str(&self.config.address).unwrap();

        russh::server::run(self.ssh_config.clone(), &address, self.clone())
            .await
            .unwrap();
    }
}

impl server::Server for Server {
    type Handler = handler::ClientHandler;

    fn new_client(&mut self, address: Option<SocketAddr>) -> handler::ClientHandler {
        let address = match address {
            Some(addr) => addr,
            None => {
                warn!("ssh address is none");
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0)
            }
        };

        handler::ClientHandler::new(
            self.service_name.clone(),
            self.service.clone(),
            address,
            self.config.clone(),
            self.main_config.clone(),
        )
    }
}
