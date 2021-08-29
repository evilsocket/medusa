use std::collections::HashMap;

use log::debug;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Command {
	parser: String,
	handler: String,
	#[serde(skip)]
	compiled: Option<Regex>,
	#[serde(skip)]
	cache: HashMap<String, String>,
}

impl Command {
	pub fn new(parser: String, handler: String) -> Self {
		Self {
			parser,
			handler,
			compiled: None,
			cache: HashMap::new(),
		}
	}

	pub fn parse(&mut self, command: &str) -> Option<String> {
		// compile regexp if needed
		if self.compiled.is_none() {
			self.compiled = Some(Regex::new(&self.parser).unwrap());
		}
		// check command for matches
		if let Some(captures) = self.compiled.as_ref().unwrap().captures(command) {
			// substitute {{$N}} tokens with matches
			let mut handler = self.handler.to_owned();
			let num = captures.len();
			for n in 1..num {
				let token = format!("{{${}}}", n).to_string();
				if handler.contains(&token) {
					handler = handler.replace(&token, &captures[n]);
				}
			}

			if let Some(out) = self.cache.get(&handler) {
				debug!("'{}' from cache: {}", &handler, out.len());
				// return from cache
				return Some(out.to_string());
			} else if handler.starts_with("@docker ") {
				let mut iter = handler.splitn(3, ' ');
				let (_, image, command) = (
					iter.next().unwrap(),
					iter.next().unwrap(),
					iter.next().unwrap(),
				);

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
