use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use log::{error, info};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
	config::{Config as MainConfig, Service},
	record,
};

use super::config::Config;

pub async fn handle(
	mut socket: tokio::net::TcpStream,
	address: SocketAddr,
	service_name: String,
	service: Arc<Mutex<Service>>,
	config: Arc<Config>,
	main_config: Arc<MainConfig>,
) {
	let mut log = record::for_address("tcp", &service_name, address);

	log.log("connected".to_owned());

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

			log.raw(buf[..n].to_vec());
			let command = String::from_utf8_lossy(&buf[..n]);

			let mut output: Option<String> = None;
			for parser in &mut service.lock().unwrap().commands {
				if let Some(out) = parser.parse(&command) {
					output = Some(out);
					break;
				}
			}

			if let Some(output) = output {
				if let Err(e) = socket.write_all(output.as_bytes()).await {
					error!("failed to send response to {}; err = {:?}", address, e);
				}
			}
		}
	}

	log.log("disconnected".to_string());

	match log.save(&main_config.records.path) {
		Ok(path) => info!("saved {} entries to {:?}", log.size(), path),
		Err(s) => error!("{}", s),
	}
}
