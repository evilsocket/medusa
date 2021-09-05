use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use clap::{AppSettings, Clap};
use futures::future;
use glob::glob;
use log::{debug, error, info};

#[derive(Clap)]
#[clap(author, about, version)]
#[clap(setting = AppSettings::ColoredHelp)]
struct Options {
    /// Path containing service YAML files.
    #[clap(short, long, default_value = "services.d")]
    pub services: String,
    /// Record files destination path.
    #[clap(short, long, default_value = "records")]
    pub records: String,
    /// Clone the network structure of a host by using shodan.io, requires --shodan-api-key
    #[clap(long)]
    pub shodan_clone: Option<String>,
    /// Shodan API key.
    #[clap(long, default_value = "")]
    pub shodan_api_key: String,
    /// Output path.
    #[clap(long, default_value = "")]
    pub output: String,
    /// Comma separated value of IP addresses to allow. If filled, any other client will be rejected.
    #[clap(long, default_value = "")]
    pub only: String,
    /// Enable debug verbosity.
    #[clap(long)]
    pub debug: bool,
}

mod config;
mod protocols;
mod record;
mod shell;
mod shodan;

fn setup() -> Options {
    let mut options: Options = Options::parse();

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(if options.debug { "debug" } else { "info" }),
    )
    .format_module_path(false)
    .format_target(false)
    .init();

    fs::create_dir_all(&options.records).expect("could not create record path");
    fs::create_dir_all(&options.services).expect("could not create services path");
    if !options.output.is_empty() {
        fs::create_dir_all(&options.output).expect("could not create output path");
        options.output = fs::canonicalize(&options.output)
            .expect("could not canonicalize output path")
            .to_str()
            .unwrap()
            .to_owned();
    }

    options.records = fs::canonicalize(&options.records)
        .expect("could not canonicalize records path")
        .to_str()
        .unwrap()
        .to_owned();

    options.services = fs::canonicalize(&options.services)
        .expect("could not canonicalize services path")
        .to_str()
        .unwrap()
        .to_owned();

    options
}

fn load_services(options: &Options) -> config::Config {
    debug!("loading services from {} ...", &options.services);

    let mut config = config::Config::new();

    config.records.path = options.records.to_string();

    if !options.only.is_empty() {
        config.only = options
            .only
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect();
    }

    for entry in glob(&format!("{}/**/*.yml", options.services)).unwrap() {
        match entry {
            Ok(path) => {
                debug!("loading {}", path.display());

                let service_name = path
                    .to_str()
                    .unwrap()
                    .replace(&options.services, "")
                    .trim_start_matches('/')
                    .trim_end_matches(".yml")
                    .replace("/", "-")
                    .to_string();

                let data = fs::read_to_string(&path)
                    .map_err(|e| format!("error reading service file {:?}: {}", &path, e))
                    .unwrap();

                let service: config::Service = serde_yaml::from_str(&data)
                    .map_err(|e| format!("error parsing service file {:?}: {}", &path, e))
                    .unwrap();

                config.services.insert(service_name, service);
            }
            Err(e) => error!("{:?}", e),
        }
    }

    config
}

#[tokio::main(flavor = "multi_thread", worker_threads = 32)]
async fn main() {
    let options = setup();

    if let Some(host) = options.shodan_clone {
        shodan::clone(&host, &options.shodan_api_key, &options.output).await;
        return;
    }

    let config = load_services(&options);

    let mut services = HashMap::new();
    let mut futures = Vec::new();

    for (name, service_config) in &config.services {
        debug!("building service {} handler", name);
        match protocols::factory(
            &service_config.proto,
            name,
            Arc::new(Mutex::new(service_config.clone())),
            config.clone(),
        ) {
            Ok(service) => {
                services.insert(name, (service, service_config));
            }
            Err(e) => {
                error!("{}", e);
                return;
            }
        }
    }

    if services.is_empty() {
        error!("no services found in {}", options.services);
    } else {
        let mut ports: Vec<u16> = vec![];

        for (name, (service, service_config)) in &services {
            info!(
                "starting {} on {} ({}) ...",
                name, service_config.address, service_config.proto
            );
            futures.push(service.run());

            let addr: SocketAddr = service_config
                .address
                .parse()
                .expect("could not parse service address");

            ports.push(addr.port());
        }

        if futures.len() > 1 {
            ports.sort_unstable();
            info!(
                "all services started on ports: {}",
                ports
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            );
        } else {
            info!("service started");
        }

        future::join_all(futures).await;
    }
}
