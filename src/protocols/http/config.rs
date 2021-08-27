use crate::config::Service;

pub fn from_service(svc: &Service) -> Config {
	let address = svc.address.to_owned();
	let headers = svc.strings("headers", vec![]);

	Config { address, headers }
}

#[derive(Clone, Debug)]
pub struct Config {
	pub address: String,
	pub headers: Vec<String>,
}
