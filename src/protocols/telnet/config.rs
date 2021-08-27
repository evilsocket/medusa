use crate::config::Service;

const DEFAULT_BANNER: &str = "server v1.0";
const DEFAULT_LOGIN_PROMPT: &str = "login: ";
const DEFAULT_PASSWORD_PROMPT: &str = "password: ";
const DEFAULT_PROMPT: &str = "# ";
const DEFAULT_TIMEOUT: u64 = 10;

pub fn from_service(svc: &Service) -> Config {
	let address = svc.address.to_owned();
	let banner = svc.string("banner", DEFAULT_BANNER);
	let login_prompt = svc.string("login_prompt", DEFAULT_LOGIN_PROMPT);
	let password_prompt = svc.string("password_prompt", DEFAULT_PASSWORD_PROMPT);
	let prompt = svc.string("prompt", DEFAULT_PROMPT);
	let timeout = svc.unsigned("timeout", DEFAULT_TIMEOUT);

	Config {
		address,
		banner,
		login_prompt,
		password_prompt,
		prompt,
		timeout,
	}
}

#[derive(Clone, Debug)]
pub struct Config {
	pub address: String,
	pub banner: String,
	pub login_prompt: String,
	pub password_prompt: String,
	pub prompt: String,
	pub timeout: u64,
}
