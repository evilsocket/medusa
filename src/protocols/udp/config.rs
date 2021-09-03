use crate::config::Service;

const DEFAULT_BANNER: &str = "hi";
const DEFAULT_TIMEOUT: u64 = 10;

pub fn from_service(svc: &Service) -> Config {
	let address = svc.address.to_owned();
	let banner = svc.string("banner", DEFAULT_BANNER);
	let timeout = svc.unsigned("timeout", DEFAULT_TIMEOUT);

	Config {
		address,
		banner,
		timeout,
	}
}

#[derive(Clone, Debug)]
pub struct Config {
	pub address: String,
	pub banner: String,
	pub timeout: u64,
}
