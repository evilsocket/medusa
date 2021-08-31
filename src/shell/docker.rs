use log::debug;

pub fn exec(container_id: &str, command: &str) -> Result<String, String> {
	debug!(
		"running command '{}' inside container '{}'",
		command, container_id
	);

	let args = vec!["exec", container_id, "sh", "-c", command];

	debug!("docker {:?}", args);
	// TODO: replace with https://github.com/softprops/shiplift
	let output = std::process::Command::new("docker")
		.args(&args)
		.output()
		.unwrap();

	let mut data = String::from_utf8_lossy(&output.stdout).to_string();

	data += String::from_utf8_lossy(&output.stderr).trim();
	data = data.replace("\n", "\r\n");

	debug!(
		"{}",
		if data.len() <= 100 {
			&data
		} else {
			&data[..100]
		}
	);

	Ok(data)
}
