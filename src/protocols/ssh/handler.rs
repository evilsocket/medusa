use std::sync::{Arc, Mutex};

use futures::future;
use hex_slice::AsHex;
use log::{error, info, warn};
use russh::{
    server::{self, Auth, Session},
    ChannelId, CryptoVec,
};

use crate::{
    config::{Config as MainConfig, Service},
    protocols::ssh::config::Config,
    record,
    shell::handler::EXIT_HANDLER_TOKEN,
};

pub struct ClientHandler {
    log: record::Record,
    address: std::net::SocketAddr,
    service: Arc<Mutex<Service>>,
    config: Arc<MainConfig>,
    banner: CryptoVec,
    prompt: CryptoVec,
    line_break: CryptoVec,
    command: Vec<u8>,
}

impl ClientHandler {
    pub fn new(
        service_name: String,
        service: Arc<Mutex<Service>>,
        address: std::net::SocketAddr,
        config: Arc<Config>,
        main_config: Arc<MainConfig>,
    ) -> Self {
        let mut log = record::for_address("ssh", &service_name, address);

        log.log("connected".to_owned());

        Self {
            banner: CryptoVec::from_slice(config.banner.as_bytes()),
            prompt: CryptoVec::from_slice(config.prompt.as_bytes()),
            line_break: CryptoVec::from_slice(b"\r\n"),
            log,
            address,
            config: main_config,
            service,
            command: vec![],
        }
    }

    fn on_command(&mut self, command: String, channel: ChannelId, session: &mut Session) -> bool {
        self.log.command(command.clone());

        let mut output: Option<String> = None;

        for parser in &mut self.service.lock().unwrap().commands {
            if let Some(out) = parser.parse(&command) {
                output = Some(out);
                break;
            }
        }

        if let Some(output) = output {
            if output == EXIT_HANDLER_TOKEN {
                return true;
            } else {
                session.data(channel, self.line_break.clone());
                session.data(channel, CryptoVec::from_slice(output.as_bytes()));
            }
        } else {
            session.data(
                channel,
                CryptoVec::from_slice(
                    format!(
                        "\r\nsh: command not found: {:?}",
                        command.split(' ').collect::<Vec<&str>>()[0]
                    )
                    .as_bytes(),
                ),
            );
        }

        session.data(channel, self.line_break.clone());
        session.data(channel, self.prompt.clone());

        false
    }

    fn auth_policy(&self) -> Auth {
        if self.config.is_allowed_ip(&self.address.ip()) {
            Auth::Accept
        } else {
            Auth::Reject {
                proceed_with_methods: None,
            }
        }
    }
}

impl Drop for ClientHandler {
    fn drop(&mut self) {
        self.log.log("disconnected".to_owned());

        match self.log.save(&self.config.records.path) {
            Ok(path) => info!("saved {} entries to {:?}", self.log.size(), path),
            Err(s) => error!("{}", s),
        }
    }
}

impl server::Handler for ClientHandler {
    type Error = anyhow::Error;
    type FutureAuth = future::Ready<Result<(Self, server::Auth), anyhow::Error>>;
    type FutureUnit = future::Ready<Result<(Self, Session), anyhow::Error>>;
    type FutureBool = future::Ready<Result<(Self, Session, bool), anyhow::Error>>;

    fn finished_auth(self, auth: Auth) -> Self::FutureAuth {
        if let Auth::Reject {
            proceed_with_methods: _,
        } = auth
        {
            warn!("rejecting {}", self.address);
        }

        future::ready(Ok((self, auth)))
    }

    fn finished_bool(self, b: bool, s: Session) -> Self::FutureBool {
        future::ready(Ok((self, s, b)))
    }

    fn finished(self, s: Session) -> Self::FutureUnit {
        future::ready(Ok((self, s)))
    }

    fn auth_none(mut self, user: &str) -> Self::FutureAuth {
        let policy = self.auth_policy();

        self.log.auth(user.to_string(), None, None);
        self.finished_auth(policy)
    }

    fn auth_password(mut self, user: &str, password: &str) -> Self::FutureAuth {
        let policy = self.auth_policy();

        self.log
            .auth(user.to_string(), Some(password.to_string()), None);
        self.finished_auth(policy)
    }

    fn auth_publickey(
        mut self,
        user: &str,
        public_key: &russh_keys::key::PublicKey,
    ) -> Self::FutureAuth {
        let policy = self.auth_policy();

        self.log
            .auth(user.to_string(), None, Some(public_key.fingerprint()));
        self.finished_auth(policy)
    }

    fn auth_keyboard_interactive(
        mut self,
        user: &str,
        _submethods: &str,
        _: Option<russh::server::Response>,
    ) -> Self::FutureAuth {
        let policy = self.auth_policy();

        self.log.auth(user.to_string(), None, None);
        self.finished_auth(policy)
    }

    fn channel_open_x11(
        mut self,
        _channel: ChannelId,
        originator_address: &str,
        originator_port: u32,
        session: Session,
    ) -> Self::FutureBool {
        self.log.log(format!(
            "channel open x11 {}:{}",
            originator_address, originator_port
        ));

        self.finished_bool(true, session)
    }

    fn channel_open_direct_tcpip(
        mut self,
        _channel: ChannelId,
        host_to_connect: &str,
        port_to_connect: u32,
        originator_address: &str,
        originator_port: u32,
        session: Session,
    ) -> Self::FutureBool {
        self.log.log(format!(
            "channel open direct tcpip {}:{} -> {}:{}",
            originator_address, originator_port, host_to_connect, port_to_connect,
        ));

        self.finished_bool(true, session)
    }

    fn channel_open_session(mut self, _channel: ChannelId, session: Session) -> Self::FutureBool {
        self.log.log("session start".to_string());

        self.finished_bool(true, session)
    }

    fn channel_close(mut self, _channel: ChannelId, session: Session) -> Self::FutureUnit {
        self.log.log("channel close".to_string());

        self.finished(session)
    }

    fn channel_eof(mut self, _channel: ChannelId, session: Session) -> Self::FutureUnit {
        self.log.log("channel eof".to_string());

        self.finished(session)
    }

    fn shell_request(mut self, channel: ChannelId, session: Session) -> Self::FutureUnit {
        // see https://github.com/warp-tech/russh/issues/35
        let mut session = session;

        self.log.log("shell request".to_string());

        session.data(channel, self.banner.clone());
        session.data(channel, self.line_break.clone());
        session.data(channel, self.line_break.clone());
        session.data(channel, self.prompt.clone());

        self.finished(session)
    }

    fn exec_request(
        mut self,
        channel: ChannelId,
        data: &[u8],
        mut session: Session,
    ) -> Self::FutureUnit {
        let command = std::str::from_utf8(data)
            .unwrap_or(&format!("{:x}", data.as_hex()))
            .trim()
            .to_owned();

        if self.on_command(command, channel, &mut session) {
            session.close(channel);
        }

        self.finished(session)
    }

    fn subsystem_request(
        mut self,
        _channel: ChannelId,
        name: &str,
        session: Session,
    ) -> Self::FutureUnit {
        self.log.log(format!("subsystem request: '{}'", name));

        self.finished(session)
    }

    fn extended_data(
        mut self,
        _channel: ChannelId,
        _code: u32,
        data: &[u8],
        session: Session,
    ) -> Self::FutureUnit {
        self.log.raw(data.to_owned());

        self.finished(session)
    }

    fn data(mut self, channel: ChannelId, data: &[u8], mut session: Session) -> Self::FutureUnit {
        match data {
            // TODO: handle backspace
            b"\r" => {
                let command = std::str::from_utf8(&self.command)
                    .unwrap_or(&format!("{:x}", self.command.as_hex()))
                    .trim()
                    .to_owned();
                self.command.clear();

                if self.on_command(command, channel, &mut session) {
                    session.close(channel);
                    return self.finished(session);
                }
            }
            _ => {
                self.command.extend(data);
                // echo back the data so that it will be displayed on the client terminal
                session.data(channel, CryptoVec::from_slice(data));
            }
        }

        self.finished(session)
    }
}
