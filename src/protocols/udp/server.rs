use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use log::{debug, error, info, warn};

use tokio::{net::UdpSocket, time::timeout};

use crate::{
	config::{Config as MainConfig, Service},
	protocols::{Error, Protocol},
	record,
};

use super::config::{self, Config};

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
		debug!("starting udp on {} ...", &self.config.address);

		let rw_timeout = Duration::from_secs(self.config.timeout);
		let mut buf = [0; 1024];
		let listener = UdpSocket::bind(&self.config.address).await.unwrap();
		loop {
			if let Ok((size, peer)) = listener.recv_from(&mut buf).await {
				if !self.main_config.is_allowed_ip(&peer.ip()) {
					warn!("{} not allowed", peer);
					drop(peer);
					continue;
				}

				let mut log = record::for_address("udp", &self.service_name, peer);

				if !self.config.banner.is_empty() {
					if let Err(e) = timeout(
						rw_timeout,
						listener.send_to(self.config.banner.as_bytes(), &peer),
					)
					.await
					{
						error!("error sending udp banner to {:?}: {}", peer, e);
					}
				}

				log.raw(buf[..size].to_vec());

				let command = String::from_utf8_lossy(&buf[..size]);

				let mut output: Option<String> = None;
				for parser in &mut self.service.lock().unwrap().commands {
					if let Some(out) = parser.parse(&command) {
						output = Some(out);
						break;
					}
				}

				if let Some(output) = output {
					if let Err(e) =
						timeout(rw_timeout, listener.send_to(output.as_bytes(), &peer)).await
					{
						error!("error sending udp response to {:?}: {}", peer, e);
					}
				}

				match log.save(&self.main_config.records.path) {
					Ok(path) => info!("saved {} entries to {:?}", log.size(), path),
					Err(s) => error!("{}", s),
				}
			}
		}
	}
}
