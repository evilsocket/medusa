use std::{fmt, fs, net::SocketAddr, path::PathBuf, str};

use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
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
            Self::Raw(data) => {
                if let Ok(s) = str::from_utf8(data) {
                    write!(f, "raw: '{}'", s)
                } else {
                    write!(f, "raw: {:?}", data)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{}> {}", self.time, self.data)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub created_at: DateTime<Utc>,
    // server info
    pub hostname: String,
    pub protocol: String,
    pub service: String,
    // client info
    pub address: String,
    pub port: u16,
    // events
    pub entries: Vec<Entry>,
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "[{}] {} -> {} ({})",
            self.created_at, self.address, self.service, self.protocol
        )?;
        for entry in &self.entries {
            writeln!(f, "  {}", entry)?;
        }
        Ok(())
    }
}

impl Record {
    pub fn log(&mut self, text: String) {
        info!("[{}] <{}> {}", &self.service, self.address, &text);
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
        info!("[{}] <{}> {}", &self.service, self.address, &request);
        self.entries.push(Entry::new(Data::Request(request)));
    }

    pub fn command(&mut self, command: String) {
        info!("[{}] <{}> {}", &self.service, self.address, &command);
        self.entries.push(Entry::new(Data::Command(command)));
    }

    pub fn raw(&mut self, data: Vec<u8>) {
        let entry = Entry::new(Data::Raw(data));
        info!("[{}] <{}> {}", &self.service, self.address, &entry.data);
        self.entries.push(entry);
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
