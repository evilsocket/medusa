use std::fs;

use log::{debug, error, info, warn};

mod templates;

pub async fn clone(host: &str, api_key: &str, output: &str) {
	if api_key.is_empty() {
		error!("--shodan-api-key not specified.");
		return;
	}

	let mut output = output;
	if output.is_empty() {
		output = host;
	}
	fs::create_dir_all(&output).expect("could not create output path for host");

	info!("cloning host {} network profile to {} ...", host, output);

	let url = format!("https://api.shodan.io/shodan/host/{}?key={}", host, api_key);

	debug!("GET {}", url);

	let resp = reqwest::get(url)
		.await
		.expect("could not perform API request");

	if resp.status() != 200 {
		error!("{}: {}", resp.status(), resp.text().await.unwrap());
		return;
	}

	let resp = resp
		.json::<serde_json::Value>()
		.await
		.expect("could not convert response to JSON");

	let open_ports = resp
		.as_object()
		.unwrap()
		.get("data")
		.unwrap()
		.as_array()
		.unwrap();

	for port in open_ports {
		let port = port.as_object().unwrap();
		let shodan = port.get("_shodan").unwrap().as_object().unwrap();
		let module = shodan.get("module").unwrap().as_str().unwrap();
		let proto = port.get("transport").unwrap().as_str().unwrap();
		let port_num = port.get("port").unwrap().as_u64().unwrap();
		let data = port.get("data").unwrap().as_str().unwrap();

		let filename = format!("{}/{}-{}.yml", output, module, port_num);

		info!(
			"cloning {} port {} ({}, {} bytes) -> {}",
			proto,
			port_num,
			module,
			data.len(),
			filename,
		);

		let yaml = match (module, proto) {
			("ssh", _) => templates::ssh(port_num, data),
			("telnet", _) => templates::telnet(port_num, data),
			("http", _) => templates::http(port_num, data, port, false),
			("https", _) => templates::http(port_num, data, port, true),
			(_, "tcp") => templates::tcp(port_num, data),
			(_, "udp") => templates::udp(port_num, data),
			(_, _) => {
				warn!("{} ({}) is not supported, skipping", module, proto);
				continue;
			}
		};

		// FIXME: for some reason hex escape sequences are not encoded correctly and end up as \\xNN
		let yaml = yaml.replace("\\\\x", "\\x");

		fs::write(filename, yaml).expect("could not create file for service");
	}
}
