use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::command;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Service {
	pub proto: String,
	pub address: String,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	#[serde(default)]
	pub commands: Vec<command::Command>,
	#[serde(flatten)]
	pub config: HashMap<String, serde_yaml::Value>,
}

impl Service {
	pub fn string(&self, name: &str, default: &str) -> String {
		self.config
			.get(name)
			.unwrap_or(&serde_yaml::Value::String(default.to_string()))
			.as_str()
			.unwrap_or(default)
			.to_string()
	}

	pub fn unsigned(&self, name: &str, default: u64) -> u64 {
		self.config
			.get(name)
			.unwrap_or(&serde_yaml::Value::Number(default.into()))
			.as_u64()
			.unwrap_or(default)
	}

	pub fn strings(&self, name: &str, default: Vec<String>) -> Vec<String> {
		if let Some(value) = self.config.get(name) {
			if let Some(sequence) = value.as_sequence() {
				return sequence
					.iter()
					.map(|v| v.as_str().unwrap().to_string())
					.collect();
			}
		}

		default
	}
}

#[derive(Clone, Deserialize, Debug)]
pub struct Records {
	pub path: String,
}

impl Records {
	pub fn new() -> Self {
		Self {
			path: "".to_string(),
		}
	}
}

#[derive(Clone, Deserialize, Debug)]
pub struct Config {
	pub records: Records,
	pub services: HashMap<String, Service>,
}

impl Config {
	pub fn new() -> Self {
		Self {
			records: Records::new(),
			services: HashMap::new(),
		}
	}
}
