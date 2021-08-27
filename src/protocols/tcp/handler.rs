use std::net::SocketAddr;
use std::sync::Arc;

use log::{error, info};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{config::Config as MainConfig, record};

use super::config::Config;

pub async fn handle(
	mut socket: tokio::net::TcpStream,
	address: SocketAddr,
	service_name: String,
	config: Arc<Config>,
	main_config: Arc<MainConfig>,
) {
	let mut log = record::for_address("tcp", &service_name, address);

	log.text("connected".to_owned());

	let mut ok = false;

	if !config.banner.is_empty() {
		match socket.write_all(config.banner.as_bytes()).await {
			Ok(_) => match socket.write_all("\r\n".as_bytes()).await {
				Ok(_) => ok = true,
				Err(e) => error!("failed to send banner to {}; err = {:?}", address, e),
			},
			Err(e) => error!("failed to send banner to {}; err = {:?}", address, e),
		}
	}

	let mut buf = [0; 1024];

	if ok {
		loop {
			let n = match socket.read(&mut buf).await {
				Ok(n) if n == 0 => {
					break;
				}
				Ok(n) => n,
				Err(e) => {
					error!("failed to read from {}; err = {:?}", address, e);
					break;
				}
			};

			log.raw(buf[0..n].to_vec());
		}
	}

	log.text("disconnected".to_string());

	match log.save(&main_config.records.path) {
		Ok(path) => info!("saved {} entries to {:?}", log.size(), path),
		Err(s) => error!("{}", s),
	}
}
