use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use log::{debug, error, info};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
	config::{Config as MainConfig, Service},
	record,
};

use super::config::Config;

async fn login_prompt(
	config: Arc<Config>,
	socket: &mut tokio::net::TcpStream,
	address: SocketAddr,
) -> Result<Option<String>, String> {
	if !config.login_prompt.is_empty() {
		match socket.write_all(config.login_prompt.as_bytes()).await {
			Ok(_) => {}
			Err(e) => {
				return Err(format!(
					"failed to send login prompt to {}; err = {:?}",
					address, e
				));
			}
		}

		let mut buf = [0; 1024];
		let n = match socket.read(&mut buf).await {
			Ok(n) if n == 0 => return Ok(None),
			Ok(n) => n,
			Err(e) => {
				return Err(format!(
					"failed to read login from {}; err = {:?}",
					address, e
				));
			}
		};

		return Ok(Some(String::from_utf8_lossy(&buf[0..n]).trim().to_string()));
	}

	Ok(None)
}

async fn password_prompt(
	config: Arc<Config>,
	socket: &mut tokio::net::TcpStream,
	address: SocketAddr,
) -> Result<Option<String>, String> {
	if !config.password_prompt.is_empty() {
		match socket.write_all(config.password_prompt.as_bytes()).await {
			Ok(_) => {}
			Err(e) => {
				return Err(format!(
					"failed to send password prompt to {}; err = {:?}",
					address, e
				));
			}
		}

		let mut buf = [0; 1024];
		let n = match socket.read(&mut buf).await {
			Ok(n) if n == 0 => return Ok(None),
			Ok(n) => n,
			Err(e) => {
				return Err(format!(
					"failed to read password from {}; err = {:?}",
					address, e
				));
			}
		};

		return Ok(Some(String::from_utf8_lossy(&buf[0..n]).trim().to_string()));
	}

	Ok(None)
}

async fn command_prompt(
	config: Arc<Config>,
	socket: &mut tokio::net::TcpStream,
	address: SocketAddr,
) -> Result<Option<String>, String> {
	if let Err(e) = socket.write_all(config.prompt.as_bytes()).await {
		return Err(format!(
			"failed to send prompt to {}; err = {:?}",
			address, e
		));
	}

	let mut buf = [0; 1024];
	let n = match socket.read(&mut buf).await {
		Ok(n) if n == 0 => return Ok(None),
		Ok(n) => n,
		Err(e) => {
			return Err(format!(
				"failed to read command from {}; err = {:?}",
				address, e
			));
		}
	};

	Ok(Some(String::from_utf8_lossy(&buf[0..n]).trim().to_string()))
}

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

	if !config.banner.is_empty() {
		if let Err(e) = socket.write_all(config.banner.as_bytes()).await {
			error!("failed to send banner to {}; err = {:?}", address, e);
			return;
		} else if let Err(e) = socket.write_all("\r\n".as_bytes()).await {
			error!("failed to send banner to {}; err = {:?}", address, e);
			return;
		}
	}

	let username = match login_prompt(config.clone(), &mut socket, address).await {
		Ok(username) => username,
		Err(e) => {
			error!("{}", e);
			return;
		}
	};

	let password = match password_prompt(config.clone(), &mut socket, address).await {
		Ok(password) => password,
		Err(e) => {
			error!("{}", e);
			return;
		}
	};

	if let Some(user) = username {
		log.auth(user, password);
	}

	while let Ok(Some(command)) = command_prompt(config.clone(), &mut socket, address).await {
		if command.is_empty() {
			continue;
		}

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
				break;
			} else {
				match socket.write_all(output.as_bytes()).await {
					Ok(_) => {}
					Err(e) => {
						error!("failed to send output to {}; err = {:?}", address, e);
						break;
					}
				}
			}
		} else {
			debug!("'{}' command not found", command);

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
					break;
				}
			}
		}

		match socket.write_all("\r\n".as_bytes()).await {
			Ok(_) => {}
			Err(e) => {
				error!("failed to send banner to {}; err = {:?}", address, e);
				break;
			}
		}
	}

	log.text("disconnected".to_string());

	match log.save(&main_config.records.path) {
		Ok(path) => info!("saved {} entries to {:?}", log.size(), path),
		Err(s) => error!("{}", s),
	}
}
