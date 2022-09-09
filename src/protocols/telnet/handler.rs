use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::{debug, error, info, warn};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};

use crate::{
    config::{Config as MainConfig, Service},
    record,
    shell::handler::EXIT_HANDLER_TOKEN,
};

use super::config::Config;

const IAC: u8 = 255;
const WONT: u8 = 252;
const ECHO: u8 = 1;

async fn login_prompt(
    config: Arc<Config>,
    socket: &mut tokio::net::TcpStream,
    address: SocketAddr,
    rw_timeout: Duration,
) -> Result<Option<String>, String> {
    if !config.login_prompt.is_empty() {
        if let Err(e) = timeout(rw_timeout, socket.write_all(config.login_prompt.as_bytes())).await
        {
            return Err(format!(
                "failed to send login prompt to {}; err = {:?}",
                address, e
            ));
        }

        let mut buf = [0; 1024];
        let n = match timeout(rw_timeout, socket.read(&mut buf)).await {
            Ok(n) => n,
            Err(e) => {
                return Err(format!(
                    "failed to read login from {}; err = {:?}",
                    address, e
                ));
            }
        };

        let n = n.unwrap_or(0);

        return if n == 0 {
            Ok(None)
        } else {
            Ok(Some(String::from_utf8_lossy(&buf[0..n]).trim().to_string()))
        };
    }

    Ok(None)
}

async fn password_prompt(
    config: Arc<Config>,
    socket: &mut tokio::net::TcpStream,
    address: SocketAddr,
    rw_timeout: Duration,
) -> Result<Option<String>, String> {
    if !config.password_prompt.is_empty() {
        if let Err(e) = timeout(
            rw_timeout,
            socket.write_all(config.password_prompt.as_bytes()),
        )
        .await
        {
            return Err(format!(
                "failed to send password prompt to {}; err = {:?}",
                address, e
            ));
        }

        let mut buf = [0; 1024];
        let n = match timeout(rw_timeout, socket.read(&mut buf)).await {
            Ok(n) => n,
            Err(e) => {
                return Err(format!(
                    "failed to read password from {}; err = {:?}",
                    address, e
                ));
            }
        };

        let n = n.unwrap_or(0);
        return if n == 0 {
            Ok(None)
        } else {
            Ok(Some(String::from_utf8_lossy(&buf[0..n]).trim().to_string()))
        };
    }

    Ok(None)
}

async fn command_prompt(
    config: Arc<Config>,
    socket: &mut tokio::net::TcpStream,
    address: SocketAddr,
    rw_timeout: Duration,
) -> Result<Option<String>, String> {
    if let Err(e) = timeout(rw_timeout, socket.write_all(config.prompt.as_bytes())).await {
        return Err(format!(
            "failed to send prompt to {}; err = {:?}",
            address, e
        ));
    }

    let mut buf = [0; 1024];
    let n = match timeout(rw_timeout, socket.read(&mut buf)).await {
        Ok(n) => n,
        Err(e) => {
            return Err(format!(
                "failed to read command from {}; err = {:?}",
                address, e
            ));
        }
    };

    let n = n.unwrap_or(0);
    return if n == 0 {
        Ok(None)
    } else {
        Ok(Some(String::from_utf8_lossy(&buf[0..n]).trim().to_string()))
    };
}

pub async fn handle(
    mut socket: tokio::net::TcpStream,
    address: SocketAddr,
    service_name: String,
    service: Arc<Mutex<Service>>,
    config: Arc<Config>,
    main_config: Arc<MainConfig>,
) {
    let mut log = record::for_address("telnet", &service_name, address);

    log.log("connected".to_owned());

    // sending initial IAC values
    let srv_iacs = vec![(ECHO, WONT)];
    for (opt, cmd) in srv_iacs {
        let buf = vec![IAC, cmd, opt];
        if let Err(e) = socket.write_all(&buf).await {
            error!("failed to send server IAC to {}; err = {:?}", address, e);
            return;
        }
    }

    // while standard telnet clients will send a few bytes of protocol at the beginning
    // most malicious clients are simple tcp-connect clients, therefore they won't send
    // anything until a prompt is shown. Just wait for 300ms and continue wether we get
    // something or not.
    let rw_timeout = Duration::from_millis(300);
    let mut buf = [0; 255];
    if let Err(e) = timeout(rw_timeout, socket.read(&mut buf)).await {
        // not fatal
        debug!("could not consume telnet first bytes from client: {}", e);
    }

    // now use the configured timeout, we should be able to send data
    let rw_timeout = Duration::from_secs(config.timeout);
    if !config.banner.is_empty() {
        if let Err(e) = timeout(rw_timeout, socket.write_all(config.banner.as_bytes())).await {
            error!("failed to send banner to {}; err = {:?}", address, e);
            return;
        } else if let Err(e) = socket.write_all("\r\n".as_bytes()).await {
            error!("failed to send banner to {}; err = {:?}", address, e);
            return;
        }
    }

    let username = match login_prompt(config.clone(), &mut socket, address, rw_timeout).await {
        Ok(username) => username,
        Err(e) => {
            // not fatal
            warn!("{}", e);
            None
        }
    };

    let password = match password_prompt(config.clone(), &mut socket, address, rw_timeout).await {
        Ok(password) => password,
        Err(e) => {
            // not fatal
            warn!("{}", e);
            None
        }
    };

    if let Some(user) = username {
        log.auth(user, password, None);
    }

    let mut keep_going = true;
    while let Ok(Some(command)) =
        command_prompt(config.clone(), &mut socket, address, rw_timeout).await
    {
        for command in command.split('\n') {
            let command = command.trim().to_string();
            if command.is_empty() {
                continue;
            }

            log.command(command.clone());

            let mut output: Option<Vec<u8>> = None;
            for parser in &mut service.lock().unwrap().commands {
                if let Some(out) = parser.parse(&command) {
                    output = Some(out);
                    break;
                }
            }

            if let Some(output) = output {
                if output == EXIT_HANDLER_TOKEN.as_bytes() {
                    keep_going = false;
                    break;
                } else if let Err(e) = timeout(rw_timeout, socket.write_all(&output)).await {
                    error!("failed to send output to {}; err = {:?}", address, e);
                    keep_going = false;
                    break;
                }
            } else {
                debug!("'{}' command not found", command);

                if let Err(e) = timeout(
                    rw_timeout,
                    socket.write_all(
                        format!(
                            "\r\nsh: command not found: {:?}",
                            command.split(' ').collect::<Vec<&str>>()[0]
                        )
                        .as_bytes(),
                    ),
                )
                .await
                {
                    error!("failed to send output to {}; err = {:?}", address, e);
                    keep_going = false;
                    break;
                }
            }

            if let Err(e) = timeout(rw_timeout, socket.write_all("\r\n".as_bytes())).await {
                error!("failed to send banner to {}; err = {:?}", address, e);
                keep_going = false;
                break;
            }
        }

        if !keep_going {
            break;
        }
    }

    log.log("disconnected".to_string());

    match log.save(&main_config.records.path) {
        Ok(path) => info!("saved {} entries to {:?}", log.size(), path),
        Err(s) => error!("{}", s),
    }
}
