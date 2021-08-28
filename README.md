Medusa is a fast and secure multi protocol honeypot that can mimic realistic devices running `ssh`, `telnet`, `http` or other `tcp` servers. 

**Work in progress.**

## Building

No precompiled binaries for the time being ...

	cargo build 

## Usage

First you need to create at least one service file. Let's begin by defining a simple SSH honeypot that accepts any combination of user and password:

	mkdir -p /path/to/services.d/
	touch /path/to/services.d/example-ssh.yml

Open `/path/to/services.d/example-ssh.yml` with your favorite editor and paste these contents:

```yaml
proto: ssh
address: '127.0.0.1:2222'
server_id: 'SSH-2.0-OpenSSH_7.2p2 Ubuntu-4ubuntu2.10'
prompt: '# '
key: '/tmp/example-ssh.key'
timeout: 15
commands:
  - parser: '^exit(\s.+)?$'
    handler: '@exit'
```

Now run:

	medusa --services "/path/to/services.d/" --records "/path/to/output/records"

This will start a single honeypoint on port 2222 and all the resulting events will be saved as JSON files in the folder indicated by `--records`.

## Commands

The previous example won't do much if somebody tries to execute actual commands. It only captures the `exit` command in order to terminate the session (via the `@exit` special handler). Let's add another command, for instance to parse simple `echo` inputs:

```yaml
proto: ssh
address: '127.0.0.1:2222'
server_id: 'SSH-2.0-OpenSSH_7.2p2 Ubuntu-4ubuntu2.10'
prompt: '# '
key: '/tmp/example-ssh.key'
timeout: 15
commands:
  - parser: '^exit(\s.+)?$'
    handler: '@exit'
  - parser: '^echo(\s"?([^"]*)"?)?$'
    handler: '{$2}'
```

The `parser` expression will now capture the `echo` command and its argument (captured as `$2`), that will be echoed back via the handler (`{$2}` is replaced with the value of `$2`).

In other cases, the handler can contain the entire output as a raw string, like for the case of an `http` service honeypot:

```yaml
proto: http 
address: '127.0.0.1:8080'
commands:
  - parser: '.*'
    handler: |
      MikroTik RouterOS:
        Version: 6.45.6
        Interfaces:
          sfp-sfpplus1
          sfp-sfpplus2
          ether1 - TCL 1 Gig WAN - INPUT
          ether2
          ether3
          ether4
          ether5 - Invetnum SR
          ether6 - SIFY INPUT 1stm
          ether7-Inventum SR 2 WAN
          ether8
          bridge1
          Alcore - 309
```

### Docker Jail

Another useful handler is `@docker`. As the name suggests it executes any shell command it receives as an argument inside a docker container, therefore we could create a "jailed" `ssh` honeypot by doing:

```yaml
proto: ssh
address: '127.0.0.1:2222'
server_id: 'SSH-2.0-OpenSSH_7.2p2 Ubuntu-4ubuntu2.10'
prompt: '# '
key: '/tmp/example-ssh.key'
timeout: 15
commands:
  - parser: '^exit(\s.+)?$'
    handler: '@exit'
  - parser: '^(.+)$'
    handler: '@docker medusajail {$1}'
```

You can create and start a jail container with:

	docker container create --name medusajail busybox tail -f /dev/null # feel free to pick any image
	docker start medusajail

This will execute any command that the client is sending on the `medusajail` container and it will transparently pass the output to the client.

Configuring a realistic docker container is beyond the purpose of this document, you can find useful images [here](https://github.com/plajjan/vrnetlab).

## Protocols

SSH server emulation (with docker jail):

```yaml
proto: ssh
address: '127.0.0.1:2222'
server_id: 'SSH-2.0-OpenSSH_7.2p2 Ubuntu-4ubuntu2.10'
prompt: '# '
key: '/tmp/example-ssh.key'
timeout: 15
commands:
  - parser: '^exit(\s.+)?$'
    handler: '@exit'
  - parser: '^(.+)$'
    handler: '@docker medusajail {$1}'
```

Telnet server emulation (with docker jail):

```yaml
proto: telnet
address: '127.0.0.1:2323'
banner: 'TNAS v1.0'
login_prompt: 'TNAS login: '
password_prompt: 'Password: '
prompt: '[admin@TNAS ~]$ '
timeout: 15
commands:
  - parser: '^exit(\s.+)?$'
    handler: '@exit'
  - parser: '^(.+)$'
    handler: '@docker medusajail {$1}'
```	

HTTP server emulation with custom headers:

```yaml
proto: http 
address: '127.0.0.1:8181'
headers:
  - 'Content-Type: text/html; charset=UTF-8'
  - 'X-Powered-By: TerraMaster'
  - 'Server: TOS/1.16.1'
  
commands:
  - parser: '.*'
    handler: |
      <!--user login-->
      <!DOCTYPE HTML>
      <html>
      <head>
          <title>TOS</title>
      </head>
      <div>
          Hello World
      </div>
      </html>
```

Other TCP servers can be simulated by exposing a banner:

```yaml
proto: tcp
address: '127.0.0.1:1723'
banner: |
  Firmware: 1
  Hostname: ASBN-BB-RT01
  Vendor: MikroTik
```

## License

Medusa was made with ♥  by [Simone Margaritelli](https://www.evilsocket.net/) and it's released under the GPL 3 license.