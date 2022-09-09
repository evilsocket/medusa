use std::collections::HashMap;

use lazy_static::lazy_static;
use log::{debug, error};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::docker;

pub const EXIT_HANDLER_TOKEN: &str = "@exit";

lazy_static! {
    static ref DOCKER_HANDLER_PARSER: Regex = Regex::new(r"^@docker\s+([^\s]+)\s+(.+)$").unwrap();
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct CommandHandler {
    parser: String,
    handler: String,
    #[serde(skip)]
    compiled: Option<Regex>,
    #[serde(skip)]
    cache: HashMap<String, Vec<u8>>,
}

impl CommandHandler {
    pub fn new(parser: String, handler: String) -> Result<Self, String> {
        let compiled = Some(
            Regex::new(&parser).map_err(|e| format!("can't compile regex '{}': {}", &parser, e))?,
        );
        Ok(Self {
            parser,
            handler,
            compiled,
            cache: HashMap::new(),
        })
    }

    fn handle_with_captures(&self, captures: &regex::Captures) -> String {
        // substitute {{$N}} tokens with matches
        let mut handler = self.handler.to_owned();
        let num = captures.len();
        for n in 1..num {
            let token = format!("{{${}}}", n).to_string();
            if handler.contains(&token) {
                handler = handler.replace(&token, &captures[n]);
            }
        }
        handler
    }

    // TODO: this should return a Result<Option<Vec<u8>>, Error>
    pub fn parse(&mut self, command: &str) -> Option<Vec<u8>> {
        if command == EXIT_HANDLER_TOKEN {
            return Some(command.as_bytes().to_owned());
        }

        if self.compiled.is_none() {
            self.compiled = Some(
                Regex::new(&self.parser)
                    .unwrap_or_else(|_| panic!("could not compile '{}'", &self.parser)),
            );
        }

        // check command for matches
        if let Some(captures) = self.compiled.as_ref().unwrap().captures(command) {
            let handler = self.handle_with_captures(&captures);

            // check cache first
            if let Some(out) = self.cache.get(&handler) {
                debug!("'{}' from cache: {}", &handler, out.len());
                return Some(out.to_owned());
            }

            // docker exec?
            if let Some(exec) = DOCKER_HANDLER_PARSER.captures(&handler) {
                let (container_id, command) = (&exec[1], &exec[2]);
                match docker::exec(container_id, command) {
                    Ok(data) => {
                        debug!("docker_exec('{}') -> {:?}", command, data);
                        self.cache.insert(handler, data.clone());
                        return Some(data);
                    }
                    Err(e) => {
                        error!(
                            "error running '{}' inside container '{}': {}",
                            command, container_id, e
                        );
                        return Some("".as_bytes().to_owned());
                    }
                }
            }

            return Some(handler.as_bytes().to_owned());
        }
        // this is not the handler you're looking for ...
        None
    }
}
