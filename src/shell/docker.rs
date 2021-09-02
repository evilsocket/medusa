use std::collections::HashMap;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;

use log::debug;

const DOCKER_SOCKET: &str = "/var/run/docker.sock";

pub fn exec(container_id: &str, command: &str) -> Result<String, String> {
	debug!(
		"running command '{}' inside container '{}'",
		command, container_id
	);

	// create exec operation
	let mut stream = UnixStream::connect(DOCKER_SOCKET)
		.map_err(|e| format!("could not connect to {}: {}", DOCKER_SOCKET, e))?;

	let content = format!(
		"{{
		\"AttachStdout\": true,
		\"Tty\": true,
		\"Cmd\": [ \"sh\", \"-c\", {}]
	  }}",
		serde_json::to_string(command).unwrap()
	);
	let request = format!("POST /containers/{}/exec HTTP/1.0\r\n", container_id)
		+ "Content-Type: application/json\r\n"
		+ &format!("Content-Length: {}\r\n", content.len())
		+ &format!("\r\n{}", content);

	debug!("{}\n\n", &request);
	stream
		.write_all(request.as_bytes())
		.map_err(|e| format!("could not write to {}: {}", DOCKER_SOCKET, e))?;

	let mut response = String::new();
	stream
		.read_to_string(&mut response)
		.map_err(|e| format!("could not read from {}: {}", DOCKER_SOCKET, e))?;

	let response = format!("{{{}", response.splitn(2, '{').nth(1).unwrap());
	let response: HashMap<String, String> = serde_json::from_str(&response).unwrap();
	let exec_id = response.get("Id").unwrap();
	debug!("exec_id = '{}'", exec_id);

	drop(stream);

	// start exec operation by id
	let mut stream = UnixStream::connect(DOCKER_SOCKET)
		.map_err(|e| format!("could not connect to {}: {}", DOCKER_SOCKET, e))?;

	let content = "{
			\"Detach\": false,
			\"Tty\": true
		  }";
	let request = format!("POST /exec/{}/start HTTP/1.0\r\n", exec_id)
		+ "Content-Type: application/json\r\n"
		+ &format!("Content-Length: {}\r\n", content.len())
		+ &format!("\r\n{}", content);

	debug!("{}\n\n", &request);
	stream
		.write_all(request.as_bytes())
		.map_err(|e| format!("could not write to {}: {}", DOCKER_SOCKET, e))?;

	let mut response = String::new();
	stream
		.read_to_string(&mut response)
		.map_err(|e| format!("could not read from {}: {}", DOCKER_SOCKET, e))?;

	debug!("{}", &response);

	let response = response.splitn(2, "\r\n\r\n").nth(1).unwrap();

	Ok(response.trim().to_owned())
}
