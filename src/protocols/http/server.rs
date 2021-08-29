use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use log::debug;

use tokio::net::TcpListener;
use tokio_rustls::{
	rustls::{
		internal::pemfile::{certs, rsa_private_keys},
		NoClientAuth, ServerConfig,
	},
	TlsAcceptor,
};

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
	tls_acceptor: Option<TlsAcceptor>,
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
		let tls_acceptor = match config.tls {
			false => None,
			true => {
				let certs = certs(&mut BufReader::new(
					File::open(&config.cert_file).map_err(|e| e.to_string())?,
				))
				.map_err(|_| "invalid certificate")?;
				let mut keys = rsa_private_keys(&mut BufReader::new(
					File::open(&config.key_file).map_err(|e| e.to_string())?,
				))
				.map_err(|_| "invalid key")?;

				let mut config = ServerConfig::new(NoClientAuth::new());
				config
					.set_single_cert(certs, keys.remove(0))
					.map_err(|e| e.to_string())?;

				Some(TlsAcceptor::from(Arc::new(config)))
			}
		};

		Ok(Server {
			service_name,
			service,
			config,
			main_config,
			tls_acceptor,
		})
	}
}

#[async_trait]
impl Protocol for Server {
	async fn run(&self) {
		debug!(
			"starting http (tls={}) on {} ...",
			if self.config.tls { "on" } else { "off" },
			&self.config.address
		);

		let listener = TcpListener::bind(&self.config.address).await.unwrap();
		while let Ok((socket, addr)) = listener.accept().await {
			if let Some(acceptor) = &self.tls_acceptor {
				if let Ok(stream) = acceptor.accept(socket).await {
					tokio::spawn(handler::handle(
						stream,
						addr,
						self.service_name.clone(),
						self.service.clone(),
						self.config.clone(),
						self.main_config.clone(),
					));
				}
			} else {
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
}
