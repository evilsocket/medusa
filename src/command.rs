use std::collections::HashMap;

use log::{debug, info};
use regex::Regex;
use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct Command {
	parser: String,
	handler: String,
	#[serde(skip)]
	compiled: Option<Regex>,
	#[serde(skip)]
	cache: HashMap<String, String>,
}

impl Command {
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
			} else if handler.starts_with("@shell ") {
				// run as a shell command
				let args: Vec<_> = handler.trim_start_matches("@shell ").split(' ').collect();

				debug!("args={:?}", args);
				let output = std::process::Command::new(args[0])
					.args(&args[1..])
					.output()
					.unwrap();

				// cache and return stderr if not empty
				let err = String::from_utf8_lossy(&output.stderr).trim().to_owned();
				if !err.is_empty() {
					self.cache.insert(handler, err.clone());
					return Some(err);
				}

				// cache and return stdolut
				let output = String::from_utf8_lossy(&output.stdout).trim().to_owned();
				let output = output.replace("\n", "\r\n");

				self.cache.insert(handler, output.clone());
				return Some(output);
			}

			return Some(handler);
		}
		// this is not the handler you're looking for ...
		None
	}
}
