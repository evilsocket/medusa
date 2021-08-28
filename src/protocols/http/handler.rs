use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use log::{error, info};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
	config::{Config as MainConfig, Service},
	record,
};

use super::config::Config;

fn response(code: u32, message: &str, headers: &[String], data: Option<String>) -> String {
	let mut resp = format!("HTTP/1.0 {} {}\r\nConnection: close\r\n", code, message);

	for header in headers {
		resp += &format!("{}\r\n", header.trim());
	}

	if let Some(data) = data {
		resp += &format!("Content-length: {}\r\n", data.len());
		resp += "\r\n";
		resp += &data;
	} else {
		resp += "Content-length: 0\r\n\r\n";
	}

	resp
}

pub async fn handle(
	mut socket: tokio::net::TcpStream,
	address: SocketAddr,
	service_name: String,
	service: Arc<Mutex<Service>>,
	config: Arc<Config>,
	main_config: Arc<MainConfig>,
) {
	let mut log = record::for_address("http", &service_name, address);

	log.text("connected".to_owned());

	let mut buf = [0; 8192];

	let n = match socket.read(&mut buf).await {
		Ok(n) => n,
		Err(e) => {
			error!("failed to read request from {}; err = {:?}", address, e);
			0
		}
	};

	if n > 0 {
		let request = String::from_utf8_lossy(&buf[0..n]).trim().to_string();

		log.request(request.clone());

		let mut output: Option<String> = None;
		{
			let mut svc = service.lock().unwrap();
			for parser in &mut svc.commands {
				if let Some(out) = parser.parse(&request) {
					output = Some(out);
					break;
				}
			}
		}

		if let Some(output) = output {
			let response = response(200, "OK", &config.headers, Some(output));
			match socket.write_all(response.as_bytes()).await {
				Ok(_) => {}
				Err(e) => {
					error!("failed to send response to {}; err = {:?}", address, e);
				}
			}
		} else {
			let response = response(404, "Not Found", &config.headers, None);
			match socket.write_all(response.as_bytes()).await {
				Ok(_) => {}
				Err(e) => {
					error!("failed to send 404 response to {}; err = {:?}", address, e);
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
