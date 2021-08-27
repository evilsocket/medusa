use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use log::debug;

use tokio::net::TcpListener;

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
	main_config: Arc<MainConfig>,
}

impl Server {
	pub fn new(
		service_name: String,
		service: Arc<Mutex<Service>>,
		main_config: MainConfig,
	) -> Result<Self, Error> {
		let config = config::from_service(service.lock().as_ref().unwrap());
		let config = Arc::new(config);
		let main_config = Arc::new(main_config);

		Ok(Server {
			service_name,
			service,
			config,
			main_config,
		})
	}
}

#[async_trait]
impl Protocol for Server {
	async fn run(&self) {
		debug!("starting telnet on {} ...", &self.config.address);

		let listener = TcpListener::bind(&self.config.address).await.unwrap();
		while let Ok((socket, addr)) = listener.accept().await {
			tokio::spawn(handler::handle(
				socket,
				addr,
				self.service_name.clone(),
				self.service.clone(),
				self.config.clone(),
				self.main_config.clone(),
			));
		}
	}
}
