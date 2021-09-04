use std::{fmt, fs, net::SocketAddr, path::PathBuf, str};

use chrono::{DateTime, Utc};
use log::{debug, info};
use serde::{ser::SerializeMap, ser::Serializer, Serialize};

#[derive(Debug, Clone)]
pub enum Data {
	Authentication {
		username: String,
		password: Option<String>,
		key: Option<String>,
	},
	Log(String),
	Command(String),
	Request(String),
	Raw(Vec<u8>),
}

// we need a custom serialization strategy because many systems such as ElasticSearch go crazy
// if for different documents in the same index, the same field has a different type.
// In the case of this enum, the "data" field would always be a string (or a Vec<u8>, that apparently
// doesn't break ES) but for the Authentication case.
// In order to avoid this, we want authentication data to be serialized as a string.
impl Serialize for Data {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut map = serializer.serialize_map(Some(2))?;

		match self {
			Self::Authentication {
				username,
				password,
				key,
			} => {
				map.serialize_entry("type", "auth").unwrap();
				map.serialize_entry(
					"data",
					&format!(
						"username:{}{}{}",
						username,
						if password.is_some() {
							format!(" password:{}", password.as_ref().unwrap())
						} else {
							"".to_owned()
						},
						if key.is_some() {
							format!(" key:{}", key.as_ref().unwrap())
						} else {
							"".to_owned()
						}
					),
				)
				.unwrap();
			}
			Self::Log(s) => {
				map.serialize_entry("type", "log").unwrap();
				map.serialize_entry("data", s).unwrap();
			}
			Self::Command(s) => {
				map.serialize_entry("type", "command").unwrap();
				map.serialize_entry("data", s).unwrap();
			}
			Self::Request(s) => {
				map.serialize_entry("type", "request").unwrap();
				map.serialize_entry("data", s).unwrap();
			}
			Self::Raw(data) => {
				map.serialize_entry("type", "raw").unwrap();
				map.serialize_entry("data", data).unwrap();
			}
		};

		map.end()
	}
}

impl fmt::Display for Data {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Authentication {
				username,
				password,
				key,
			} => {
				write!(
					f,
					"authentication: user={} pass={:?} key={:?}",
					username, password, key
				)
			}
			Self::Log(s) => write!(f, "{}", s),
			Self::Command(s) => write!(f, "command: {}", s),
			Self::Request(s) => write!(f, "request: {}", s),
			Self::Raw(data) => write!(f, "raw: {:?}", str::from_utf8(data)),
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
	// server info
	hostname: String,
	protocol: String,
	service: String,
	// client info
	address: String,
	port: u16,
	// events
	entries: Vec<Entry>,
}

impl Record {
	pub fn log(&mut self, text: String) {
		debug!("[{}] <{}> {}", &self.service, self.address, &text);
		self.entries.push(Entry::new(Data::Log(text)));
	}

	pub fn auth(&mut self, username: String, password: Option<String>, key: Option<String>) {
		info!(
			"[{}] <{}> AUTH: username:{}{}{}",
			&self.service,
			self.address,
			username,
			if password.is_some() {
				format!(" password:{}", password.as_ref().unwrap())
			} else {
				"".to_owned()
			},
			if key.is_some() {
				format!(" key:{}", key.as_ref().unwrap())
			} else {
				"".to_owned()
			}
		);
		self.entries.push(Entry::new(Data::Authentication {
			username,
			password,
			key,
		}));
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

	fn path(&self, folder: &str) -> PathBuf {
		let mut path = PathBuf::from(folder);

		path.push(&self.address);
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

		let data = serde_json::to_string_pretty(self)
			.map_err(|e| format!("could not convert record to json: {}", e))?;

		if let Err(e) = fs::write(&path, data) {
			return Err(format!("could not write record: {}", e));
		}

		Ok(path)
	}
}

pub fn for_address(protocol: &str, service: &str, address: SocketAddr) -> Record {
	let hostname = gethostname::gethostname()
		.to_str()
		.unwrap_or("could not detect hostname")
		.to_owned();
	Record {
		created_at: Utc::now(),
		protocol: protocol.to_owned(),
		service: service.to_owned(),
		entries: Vec::new(),
		address: address.ip().to_string(),
		port: address.port(),
		hostname,
	}
}
