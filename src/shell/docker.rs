use std::collections::HashMap;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use log::debug;

const DOCKER_SOCKET: &str = "/var/run/docker.sock";

fn do_request<S>(request: &str, stream: &mut S) -> Result<Vec<u8>, String>
where
    S: Read + Write,
{
    debug!("{}\n\n", request);
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("could not write to {}: {}", DOCKER_SOCKET, e))?;

    let mut response = vec![];
    stream
        .read_to_end(&mut response)
        .map_err(|e| format!("could not read from {}: {}", DOCKER_SOCKET, e))?;

    debug!("{:?}", &response);

    Ok(response)
}

fn create_exec(container_id: &str, command: &str) -> Result<String, String> {
    debug!(
        "creating exec operation for container {}: {}",
        container_id, command
    );

    let mut stream = UnixStream::connect(DOCKER_SOCKET)
        .map_err(|e| format!("could not connect to {}: {}", DOCKER_SOCKET, e))?;

    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("could not set read timeout for {}: {}", DOCKER_SOCKET, e))?;

    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("could not set write timeout for {}: {}", DOCKER_SOCKET, e))?;

    let content = format!(
        "{{
		\"AttachStdout\": true,
		\"Tty\": false,
		\"Cmd\": [ \"sh\", \"-c\", {}]
	  }}",
        serde_json::to_string(command).unwrap()
    );

    let request = format!("POST /containers/{}/exec HTTP/1.0\r\n", container_id)
        + "Content-Type: application/json\r\n"
        + &format!("Content-Length: {}\r\n", content.len())
        + &format!("\r\n{}", content);

    let response = do_request(&request, &mut stream)?;
    let response = String::from_utf8_lossy(&response).to_string();

    // split headers from response body
    let response = format!("{{{}", response.split_once('{').unwrap().1);
    // parse as generic hashmap
    let response: HashMap<String, String> = serde_json::from_str(&response).unwrap();
    // get exec id
    if let Some(exec_id) = response.get("Id") {
        debug!("exec_id = '{}'", &exec_id);
        return Ok(exec_id.to_owned());
    }

    Err(format!("{:?}", response))
}

fn do_exec(exec_id: &str) -> Result<Vec<u8>, String> {
    debug!("dispatching exec operation {}", exec_id);

    let mut stream = UnixStream::connect(DOCKER_SOCKET)
        .map_err(|e| format!("could not connect to {}: {}", DOCKER_SOCKET, e))?;

    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("could not set read timeout for {}: {}", DOCKER_SOCKET, e))?;

    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("could not set write timeout for {}: {}", DOCKER_SOCKET, e))?;

    let content = "{
			\"Detach\": false,
			\"Tty\": false
		  }";
    let request = format!("POST /exec/{}/start HTTP/1.0\r\n", exec_id)
        + "Content-Type: application/json\r\n"
        + &format!("Content-Length: {}\r\n", content.len())
        + &format!("\r\n{}", content);

    let response = do_request(&request, &mut stream)?;
    // split headers and response body
    let pattern = "\r\n\r\n".as_bytes();
    let idx = response
        .windows(pattern.len())
        .position(|window| window == pattern)
        .unwrap()
        + pattern.len();

    Ok(response[idx..].to_vec())
}

pub fn exec(container_id: &str, command: &str) -> Result<Vec<u8>, String> {
    debug!(
        "running command '{}' inside container '{}'",
        command, container_id
    );

    // create exec operation
    let exec_id = create_exec(container_id, command)?;

    // start exec operation by id
    do_exec(&exec_id)
}
