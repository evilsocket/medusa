use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use log::{debug, warn};

use tokio::net::TcpListener;
use tokio_rustls::{
	rustls::{
		internal::pemfile::{certs, pkcs8_private_keys, rsa_private_keys},
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
	fn configure_tls(config: &Config) -> Result<Option<TlsAcceptor>, Error> {
		let mut tls: Option<TlsAcceptor> = None;
		if config.tls {
			let certs = certs(&mut BufReader::new(File::open(&config.cert_file).map_err(
				|e| format!("could not open {}: {}", config.cert_file, e.to_string()),
			)?))
			.map_err(|_| "invalid certificate")?;

			let mut keys =
				pkcs8_private_keys(&mut BufReader::new(File::open(&config.key_file).map_err(
					|e| format!("could not open {}: {}", config.key_file, e.to_string()),
				)?))
				.map_err(|_| "invalid key")?;

			if keys.is_empty() {
				// try PKCS#1 before returning an error
				keys =
					rsa_private_keys(&mut BufReader::new(File::open(&config.key_file).map_err(
						|e| format!("could not open {}: {}", config.key_file, e.to_string()),
					)?))
					.map_err(|_| "invalid key")?;

				if keys.is_empty() {
					return Err(format!(
						"no valid PKCS#8 or PKCS#1 encoded keys found in {}",
						&config.key_file
					));
				}
			}

			let mut config = ServerConfig::new(NoClientAuth::new());
			config.set_single_cert(certs, keys.remove(0)).map_err(|e| {
				format!("could not set https certificate and key: {}", e.to_string())
			})?;

			tls = Some(TlsAcceptor::from(Arc::new(config)));
		}

		Ok(tls)
	}

	pub fn new(
		service_name: String,
		service: Arc<Mutex<Service>>,
		main_config: MainConfig,
	) -> Result<Self, Error> {
		let config = config::from_service(service.lock().as_ref().unwrap());
		let config = Arc::new(config);
		let main_config = Arc::new(main_config);
		let tls_acceptor = Self::configure_tls(&config)?;

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
			if !self.main_config.is_allowed_ip(&addr.ip()) {
				warn!("{} not allowed", addr);
				drop(socket);
				continue;
			}

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
