use std::fs;

use glob::glob;
use log::{debug, error};

use crate::config;
use crate::Options;

pub(crate) fn load(options: &Options) -> config::Config {
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
                    .replace('/', "-")
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
