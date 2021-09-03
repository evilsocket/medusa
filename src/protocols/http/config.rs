use crate::config::Service;

pub const DEFAULT_CERT_FILE: &str = "/tmp/medusa-https.crt";
pub const DEFAULT_KEY_FILE: &str = "/tmp/medusa-https.key";
const DEFAULT_TIMEOUT: u64 = 10;

pub fn from_service(svc: &Service) -> Config {
	let address = svc.address.to_owned();
	let headers = svc.strings("headers", vec![]);
	let tls = svc.bool("tls", false);
	let key_file = svc.string("key", DEFAULT_KEY_FILE);
	let cert_file = svc.string("certificate", DEFAULT_CERT_FILE);
	let timeout = svc.unsigned("timeout", DEFAULT_TIMEOUT);

	Config {
		address,
		headers,
		tls,
		key_file,
		cert_file,
		timeout,
	}
}

#[derive(Clone, Debug)]
pub struct Config {
	pub address: String,
	pub headers: Vec<String>,
	pub tls: bool,
	pub key_file: String,
	pub cert_file: String,
	pub timeout: u64,
}
