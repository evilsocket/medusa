use std::time::{Duration, Instant};

use log::{debug, info};
use russh::SshId;
use russh_keys::key::{KeyPair, SignatureHash};

use crate::{config::Service, protocols::Error};

pub const DEFAULT_ID: &str = "SSH-2.0-OpenSSH_7.2p2 Ubuntu-4ubuntu2.10";
pub const DEFAULT_BANNER: &str = "Last login: Mon Sep  5 14:12:09 2022 from 127.0.0.1";
pub const DEFAULT_PROMPT: &str = "# ";
pub const DEFAULT_TIMEOUT: u64 = 10;
pub const DEFAULT_RSA_BITS: usize = 1024;

pub fn from_service(svc: &Service) -> Config {
    let address = svc.address.to_owned();
    let server_id = svc.string("server_id", DEFAULT_ID);
    let server_id_raw = svc.string("server_id_raw", "");
    let banner = svc.string("banner", DEFAULT_BANNER);
    let prompt = svc.string("prompt", DEFAULT_PROMPT);
    let timeout = svc.unsigned("timeout", DEFAULT_TIMEOUT);

    Config {
        address,
        server_id,
        server_id_raw,
        banner,
        prompt,
        timeout,
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub address: String,
    pub server_id: String,
    pub server_id_raw: String,
    pub banner: String,
    pub prompt: String,
    pub timeout: u64,
}

impl Config {
    pub fn to_ssh_config(&self) -> Result<russh::server::Config, Error> {
        let mut ssh_config = russh::server::Config::default();

        info!("generating ssh keys ...");

        let start = Instant::now();

        ssh_config.keys.push(KeyPair::generate_ed25519().unwrap());
        ssh_config
            .keys
            .push(KeyPair::generate_rsa(DEFAULT_RSA_BITS, SignatureHash::SHA1).unwrap());
        ssh_config
            .keys
            .push(KeyPair::generate_rsa(DEFAULT_RSA_BITS, SignatureHash::SHA2_256).unwrap());
        ssh_config
            .keys
            .push(KeyPair::generate_rsa(DEFAULT_RSA_BITS, SignatureHash::SHA2_512).unwrap());

        info!("ssh keys generated in {:?}", start.elapsed());

        // see https://github.com/evilsocket/medusa/issues/3
        ssh_config.server_id = if self.server_id_raw.is_empty() {
            SshId::Standard(self.server_id.to_owned())
        } else {
            SshId::Raw(self.server_id_raw.to_owned())
        };

        debug!("ssh server id: {:?}", &ssh_config.server_id);

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
