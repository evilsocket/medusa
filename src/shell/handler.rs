use std::collections::HashMap;

use lazy_static::lazy_static;
use log::debug;
use regex::Regex;
use serde::{Deserialize, Serialize};

pub const EXIT_HANDLER_TOKEN: &str = "@exit";

lazy_static! {
	static ref DOCKER_HANDLER_PARSER: Regex = Regex::new(r"^@docker\s+([^\s]+)\s+(.+)$").unwrap();
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct CommandHandler {
	parser: String,
	handler: String,
	#[serde(skip)]
	compiled: Option<Regex>,
	#[serde(skip)]
	cache: HashMap<String, String>,
}

impl CommandHandler {
	pub fn new(parser: String, handler: String) -> Result<Self, String> {
		let compiled = Some(
			Regex::new(&parser).map_err(|e| format!("can't compile regex '{}': {}", &parser, e))?,
		);
		Ok(Self {
			parser,
			handler,
			compiled,
			cache: HashMap::new(),
		})
	}

	fn handle_with_captures(&self, captures: &regex::Captures) -> String {
		// substitute {{$N}} tokens with matches
		let mut handler = self.handler.to_owned();
		let num = captures.len();
		for n in 1..num {
			let token = format!("{{${}}}", n).to_string();
			if handler.contains(&token) {
				handler = handler.replace(&token, &captures[n]);
			}
		}
		handler
	}

	pub fn parse(&mut self, command: &str) -> Option<String> {
		// check command for matches
		if let Some(captures) = self.compiled.as_ref().unwrap().captures(command) {
			let handler = self.handle_with_captures(&captures);

			// check cache first
			if let Some(out) = self.cache.get(&handler) {
				debug!("'{}' from cache: {}", &handler, out.len());
				return Some(out.to_string());
			}

			// docker exec?
			if let Some(exec) = DOCKER_HANDLER_PARSER.captures(&handler) {
				let (image, command) = (&exec[1], &exec[2]);
				let args = vec!["exec", image, "sh", "-c", command];

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

				self.cache.insert(handler, data.clone());
				return Some(data);
			}

			return Some(handler);
		}
		// this is not the handler you're looking for ...
		None
	}
}
