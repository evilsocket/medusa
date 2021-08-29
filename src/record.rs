use std::{fmt, fs, net::SocketAddr, path::PathBuf, str};

use chrono::{DateTime, Utc};
use log::{debug, info};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum Data {
	Authentication(String, Option<String>),
	Text(String),
	Command(String),
	Request(String),
	Raw(Vec<u8>),
}

impl fmt::Display for Data {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Authentication(user, pass) => write!(f, "user={} pass={:?}", user, pass),
			Self::Text(s) => write!(f, "{}", s),
			Self::Command(s) => write!(f, "{}", s),
			Self::Request(s) => write!(f, "{}", s),
			Self::Raw(data) => write!(f, "{:?}", str::from_utf8(data)),
		}
	}
}

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
	pub time: DateTime<Utc>,
	pub data: Data,
}

impl Entry {
	pub fn new(data: Data) -> Self {
		Self {
			time: Utc::now(),
			data,
		}
	}
}

#[derive(Debug, Clone, Serialize)]
pub struct Record {
	created_at: DateTime<Utc>,
	protocol: String,
	service: String,
	address: SocketAddr,
	entries: Vec<Entry>,
}

impl Record {
	pub fn text(&mut self, text: String) {
		debug!("[{}] <{}> {}", &self.service, self.address, &text);
		self.entries.push(Entry::new(Data::Text(text)));
	}

	pub fn auth(&mut self, user: String, pass: Option<String>) {
		debug!(
			"[{}] <{}> user={} pass={:?}",
			&self.service, self.address, user, pass
		);
		self.entries
			.push(Entry::new(Data::Authentication(user, pass)));
	}

	pub fn request(&mut self, request: String) {
		debug!("[{}] <{}> {}", &self.service, self.address, &request);
		self.entries.push(Entry::new(Data::Request(request)));
	}

	pub fn command(&mut self, command: String) {
		info!("[{}] <{}> {}", &self.service, self.address, &command);
		self.entries.push(Entry::new(Data::Command(command)));
	}

	pub fn raw(&mut self, data: Vec<u8>) {
		debug!(
			"[{}] <{}> {:?} -> {:?}",
			&self.service,
			self.address,
			&data,
			String::from_utf8(data.clone())
		);
		self.entries.push(Entry::new(Data::Raw(data)));
	}

	pub fn size(&self) -> usize {
		self.entries.len()
	}

	fn reduce(&mut self) {
		let mut entries: Vec<Entry> = Vec::new();
		let mut data: Vec<u8> = Vec::new();

		// FIXME: only time adjacent bytes should be concatenated.

		for entry in &self.entries {
			match &entry.data {
				Data::Raw(raw) => {
					data.extend(raw);
				}
				_ => {
					if !data.is_empty() {
						entries.push(Entry::new(Data::Raw(data.clone())));
						data.clear();
					}
					entries.push(entry.clone());
				}
			}
		}

		if !data.is_empty() {
			entries.push(Entry::new(Data::Raw(data.clone())));
			data.clear();
		}

		self.entries = entries;
	}

	fn path(&self, folder: &str) -> PathBuf {
		let mut path = PathBuf::from(folder);

		path.push(self.address.ip().to_string());
		path.push(&self.service);

		path.push(format!(
			"{}.json",
			self.created_at.format("%+") // ISO 8601 / RFC 3339
		));

		path
	}

	pub fn save(&mut self, folder: &str) -> Result<PathBuf, String> {
		let path = self.path(folder);
		let parent = match path.parent() {
			Some(parent) => parent,
			None => return Err(format!("could not get parent folder of {:?}", path)),
		};

		fs::create_dir_all(parent).map_err(|e| format!("could not create {:?}: {}", parent, e))?;

		self.reduce();

		let data = serde_json::to_string_pretty(self)
			.map_err(|e| format!("could not convert record to json: {}", e))?;

		fs::write(&path, data).expect("could not write record");

		Ok(path)
	}
}

pub fn for_address(protocol: &str, service: &str, address: SocketAddr) -> Record {
	Record {
		created_at: Utc::now(),
		protocol: protocol.to_owned(),
		service: service.to_owned(),
		entries: Vec::new(),
		address,
	}
}
