use crate::config::Service;

const DEFAULT_BANNER: &str = "hi";

pub fn from_service(svc: &Service) -> Config {
	let address = svc.address.to_owned();
	let banner = svc.string("banner", DEFAULT_BANNER);

	Config { address, banner }
}

#[derive(Clone, Debug)]
pub struct Config {
	pub address: String,
	pub banner: String,
}
