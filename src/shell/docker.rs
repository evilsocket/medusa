use std::collections::HashMap;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;

use log::debug;

const DOCKER_SOCKET: &str = "/var/run/docker.sock";

fn do_request<S>(request: &str, stream: &mut S) -> Result<String, String>
where
	S: Read + Write,
{
	debug!("{}\n\n", request);
	stream
		.write_all(request.as_bytes())
		.map_err(|e| format!("could not write to {}: {}", DOCKER_SOCKET, e))?;

	let mut response = String::new();
	stream
		.read_to_string(&mut response)
		.map_err(|e| format!("could not read from {}: {}", DOCKER_SOCKET, e))?;

	debug!("{}", &response);

	Ok(response)
}

fn create_exec(container_id: &str, command: &str) -> Result<String, String> {
	debug!(
		"creating exec operation for container {}: {}",
		container_id, command
	);

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

	let response = do_request(&request, &mut stream)?;
	// split headers from response body
	let response = format!("{{{}", response.splitn(2, '{').nth(1).unwrap());
	// parse as generic hashmap
	let response: HashMap<String, String> = serde_json::from_str(&response).unwrap();
	// get exec id
	let exec_id = response
		.get("Id")
		.unwrap_or_else(|| panic!("{:?}", response))
		.to_owned();

	debug!("exec_id = '{}'", &exec_id);

	Ok(exec_id)
}

fn do_exec(exec_id: &str) -> Result<String, String> {
	debug!("dispatching exec operation {}", exec_id);

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

	let response = do_request(&request, &mut stream)?;
	// split headers and response body
	let response = response.splitn(2, "\r\n\r\n").nth(1).unwrap();

	Ok(response.trim().to_owned())
}

pub fn exec(container_id: &str, command: &str) -> Result<String, String> {
	debug!(
		"running command '{}' inside container '{}'",
		command, container_id
	);

	// create exec operation
	let exec_id = create_exec(container_id, command)?;

	// start exec operation by id
	do_exec(&exec_id)
}
