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
	let mut log = record::for_address("telnet", &service_name, address);

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

	while ok {
		let mut username: Option<String> = None;
		let mut password: Option<String> = None;

		if !config.login_prompt.is_empty() {
			match socket.write_all(config.login_prompt.as_bytes()).await {
				Ok(_) => {}
				Err(e) => {
					error!("failed to send login prompt to {}; err = {:?}", address, e);
					break;
				}
			}

			let n = match socket.read(&mut buf).await {
				Ok(n) if n == 0 => break,
				Ok(n) => n,
				Err(e) => {
					error!("failed to read login from {}; err = {:?}", address, e);
					break;
				}
			};

			username = Some(String::from_utf8_lossy(&buf[0..n]).trim().to_string());
		}

		if !config.password_prompt.is_empty() {
			match socket.write_all(config.password_prompt.as_bytes()).await {
				Ok(_) => {}
				Err(e) => {
					error!(
						"failed to send password prompt to {}; err = {:?}",
						address, e
					);
					break;
				}
			}

			let n = match socket.read(&mut buf).await {
				Ok(n) if n == 0 => break,
				Ok(n) => n,
				Err(e) => {
					error!("failed to read password from {}; err = {:?}", address, e);
					break;
				}
			};

			password = Some(String::from_utf8_lossy(&buf[0..n]).trim().to_string());
		}

		if let Some(user) = username {
			log.auth(user, password);
		}

		while ok {
			match socket.write_all(config.prompt.as_bytes()).await {
				Ok(_) => {}
				Err(e) => {
					error!("failed to send prompt to {}; err = {:?}", address, e);
					ok = false;
					break;
				}
			}
			let n = match socket.read(&mut buf).await {
				Ok(n) if n == 0 => {
					ok = false;
					break;
				}
				Ok(n) => n,
				Err(e) => {
					error!("failed to read password from {}; err = {:?}", address, e);
					ok = false;
					break;
				}
			};

			let command = String::from_utf8_lossy(&buf[0..n]).trim().to_string();

			log.command(command.clone());

			let mut output: Option<String> = None;
			for parser in &mut service.lock().unwrap().commands {
				if let Some(out) = parser.parse(&command) {
					output = Some(out);
					break;
				}
			}

			if let Some(output) = output {
				if output == "@exit" {
					ok = false;
				} else {
					match socket.write_all(output.as_bytes()).await {
						Ok(_) => {}
						Err(e) => {
							error!("failed to send output to {}; err = {:?}", address, e);
							ok = false;
						}
					}
				}
			} else {
				match socket
					.write_all(
						format!(
							"\r\nsh: command not found: {:?}",
							command.split(' ').collect::<Vec<&str>>()[0]
						)
						.as_bytes(),
					)
					.await
				{
					Ok(_) => {}
					Err(e) => {
						error!("failed to send output to {}; err = {:?}", address, e);
						ok = false;
					}
				}
			}

			match socket.write_all("\r\n".as_bytes()).await {
				Ok(_) => {}
				Err(e) => {
					error!("failed to send banner to {}; err = {:?}", address, e);
					ok = false;
				}
			}
		}
	}

	log.text("disconnected".to_string());

	match log.save(&main_config.records.path) {
		Ok(path) => info!("saved {} entries to {:?}", log.size(), path),
		Err(s) => error!("{}", s),
	}
}
