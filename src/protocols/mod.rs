use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::config::{Config, Service};

pub mod http;
pub mod ssh;
pub mod tcp;
pub mod telnet;
pub mod udp;

pub type Error = String;

#[async_trait]
pub trait Protocol {
    async fn run(&self);
}

pub fn factory(
    protocol_name: &str,
    service_name: &str,
    service: Arc<Mutex<Service>>,
    config: Config,
) -> Result<Box<dyn Protocol>, Error> {
    match protocol_name {
        "tcp" => match tcp::server::Server::new(service_name.to_owned(), service, config) {
            Ok(ssh) => Ok(Box::new(ssh)),
            Err(e) => Err(e),
        },
        "udp" => match udp::server::Server::new(service_name.to_owned(), service, config) {
            Ok(ssh) => Ok(Box::new(ssh)),
            Err(e) => Err(e),
        },
        "telnet" => match telnet::server::Server::new(service_name.to_owned(), service, config) {
            Ok(telnet) => Ok(Box::new(telnet)),
            Err(e) => Err(e),
        },
        "http" => match http::server::Server::new(service_name.to_owned(), service, config) {
            Ok(telnet) => Ok(Box::new(telnet)),
            Err(e) => Err(e),
        },
        "ssh" => match ssh::server::Server::new(service_name.to_owned(), service, config) {
            Ok(ssh) => Ok(Box::new(ssh)),
            Err(e) => Err(e),
        },
        _ => Err(format!("protocol '{}' is not supported", protocol_name)),
    }
}
