use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    config::Service,
    shell::handler::{CommandHandler, EXIT_HANDLER_TOKEN},
};

const HTTP_IGNORE_HEADERS: &[&str] = &["connection", "content-length", "date"];

lazy_static! {
    static ref HTTP_HEADERS_PARSER: Regex = Regex::new(r"(.+): (.+)").unwrap();
}

pub fn ssh(port_num: u64, data: &str) -> String {
    let commands =
        vec![
            CommandHandler::new(r"^exit(\s.+)?$".to_owned(), EXIT_HANDLER_TOKEN.to_owned())
                .unwrap(),
        ];

    let server_id = data.split('\n').next().unwrap();

    let mut config: HashMap<String, serde_yaml::Value> = HashMap::new();

    config.insert(
        "server_id".to_string(),
        serde_yaml::to_value(server_id).unwrap(),
    );
    config.insert("prompt".to_string(), serde_yaml::to_value("# ").unwrap());
    config.insert("timeout".to_string(), serde_yaml::to_value(15).unwrap());

    serde_yaml::to_string(&Service {
        proto: "ssh".to_owned(),
        address: format!("0.0.0.0:{}", port_num),
        commands,
        config,
    })
    .unwrap()
}

pub fn telnet(port_num: u64, data: &str) -> String {
    let commands =
        vec![
            CommandHandler::new(r"^exit(\s.+)?$".to_owned(), EXIT_HANDLER_TOKEN.to_owned())
                .unwrap(),
        ];

    let mut config: HashMap<String, serde_yaml::Value> = HashMap::new();

    config.insert("banner".to_string(), serde_yaml::to_value(data).unwrap());
    config.insert(
        "login_prompt".to_string(),
        serde_yaml::to_value("Login : ").unwrap(),
    );
    config.insert(
        "password_prompt".to_string(),
        serde_yaml::to_value("Password : ").unwrap(),
    );
    config.insert("prompt".to_string(), serde_yaml::to_value("# ").unwrap());
    config.insert("timeout".to_string(), serde_yaml::to_value(15).unwrap());

    serde_yaml::to_string(&Service {
        proto: "telnet".to_owned(),
        address: format!("0.0.0.0:{}", port_num),
        commands,
        config,
    })
    .unwrap()
}

pub fn tcp(port_num: u64, data: &str) -> String {
    let commands = vec![];

    let mut config: HashMap<String, serde_yaml::Value> = HashMap::new();

    config.insert("banner".to_string(), serde_yaml::to_value(data).unwrap());

    serde_yaml::to_string(&Service {
        proto: "tcp".to_owned(),
        address: format!("0.0.0.0:{}", port_num),
        commands,
        config,
    })
    .unwrap()
}

pub fn udp(port_num: u64, data: &str) -> String {
    let commands = vec![];

    let mut config: HashMap<String, serde_yaml::Value> = HashMap::new();

    config.insert("banner".to_string(), serde_yaml::to_value(data).unwrap());

    serde_yaml::to_string(&Service {
        proto: "udp".to_owned(),
        address: format!("0.0.0.0:{}", port_num),
        commands,
        config,
    })
    .unwrap()
}

pub fn http(
    port_num: u64,
    data: &str,
    port: &serde_json::Map<String, serde_json::Value>,
    tls: bool,
) -> String {
    let http = port.get("http").unwrap().as_object().unwrap();
    let response = http.get("html").unwrap().as_str().unwrap();

    // parse headers
    let mut headers = vec![];
    for caps in HTTP_HEADERS_PARSER.captures_iter(data) {
        let header = caps.get(1).unwrap().as_str().trim();
        let header_lwr = header.to_lowercase();
        let value = caps.get(2).unwrap().as_str().trim();

        if !HTTP_IGNORE_HEADERS.contains(&header_lwr.as_str()) {
            headers.push(format!("{}: {}", header, value));
        }
    }

    let commands = vec![CommandHandler::new(r".*".to_owned(), response.to_owned()).unwrap()];

    let mut config: HashMap<String, serde_yaml::Value> = HashMap::new();

    config.insert(
        "headers".to_string(),
        serde_yaml::to_value(headers).unwrap(),
    );

    if tls {
        config.insert("tls".to_string(), serde_yaml::to_value(true).unwrap());
        config.insert(
            "key".to_string(),
            serde_yaml::to_value(crate::protocols::http::config::DEFAULT_KEY_FILE).unwrap(),
        );
        config.insert(
            "certificate".to_string(),
            serde_yaml::to_value(crate::protocols::http::config::DEFAULT_CERT_FILE).unwrap(),
        );
    }

    serde_yaml::to_string(&Service {
        proto: "http".to_owned(),
        address: format!("0.0.0.0:{}", port_num),
        commands,
        config,
    })
    .unwrap()
}
