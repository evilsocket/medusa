use std::path::Path;
use std::time::Duration;

use log::{debug, info};
use russh_keys::{self, encode_pkcs8_pem, load_secret_key};

use crate::{config::Service, protocols::Error};

pub const DEFAULT_ID: &str = "SSH-2.0-OpenSSH_7.2p2 Ubuntu-4ubuntu2.10";
pub const DEFAULT_BANNER: &str = "Last login: Mon Sep  5 14:12:09 2022 from 127.0.0.1";
pub const DEFAULT_PROMPT: &str = "# ";
pub const DEFAULT_KEY_FILE: &str = "/tmp/medusa-ssh.key";
pub const DEFAULT_TIMEOUT: u64 = 10;

pub fn from_service(svc: &Service) -> Config {
    let address = svc.address.to_owned();
    let server_id = svc.string("server_id", DEFAULT_ID);
    let banner = svc.string("banner", DEFAULT_BANNER);
    let prompt = svc.string("prompt", DEFAULT_PROMPT);
    let key_file = svc.string("key", DEFAULT_KEY_FILE);
    let timeout = svc.unsigned("timeout", DEFAULT_TIMEOUT);

    Config {
        address,
        server_id,
        banner,
        prompt,
        key_file,
        timeout,
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub address: String,
    pub server_id: String,
    pub banner: String,
    pub prompt: String,
    pub key_file: String,
    pub timeout: u64,
}

impl Config {
    pub fn to_ssh_config(&self) -> Result<russh::server::Config, Error> {
        let mut ssh_config = russh::server::Config::default();

        let key_file = Path::new(&self.key_file);
        if key_file.exists() {
            debug!("loading key from {}", &self.key_file);
            let key = match load_secret_key(key_file, None) {
                Ok(key) => key,
                Err(e) => return Err(format!("error loading {}: {}", &self.key_file, e)),
            };

            ssh_config.keys.push(key);
        } else {
            info!("generating new key ...");

            let key = match russh_keys::key::KeyPair::generate_ed25519() {
                Some(key) => key,
                None => return Err("error generating key".to_string()),
            };

            info!("saving key to {}", &self.key_file);

            let file = match std::fs::File::create(key_file) {
                Ok(file) => file,
                Err(e) => return Err(format!("error creating {}: {}", &self.key_file, e)),
            };

            if let Err(e) = encode_pkcs8_pem(&key, file) {
                return Err(format!("error encoding key to {}: {}", &self.key_file, e));
            }

            ssh_config.keys.push(key);
        }

        ssh_config.server_id = self.server_id.to_owned();
        ssh_config.auth_rejection_time = Duration::ZERO;
        ssh_config.methods = russh::MethodSet::NONE
            | russh::MethodSet::PASSWORD
            | russh::MethodSet::PUBLICKEY
            | russh::MethodSet::HOSTBASED
            | russh::MethodSet::KEYBOARD_INTERACTIVE;
        ssh_config.connection_timeout = Some(Duration::from_secs(self.timeout));

        Ok(ssh_config)
    }
}
