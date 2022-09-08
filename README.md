# Medusa

A fast and secure multi protocol honeypot that can mimic realistic devices running `ssh`, `telnet`, `http`, `https` or any other `tcp` and `udp` servers. 

**Work in progress.**

## Building

Using Docker (recommended):

```sh
docker build -t medusa .
docker run \
  -v /path/to/services.d:/etc/medusa/services.d \
  -v /path/to/records:/var/lib/medusa/records \
  --network host \
  medusa
```

You can also bring up a system that will automatically import new records on an ElasticSearch database by using `docker-compose`. All you need to do, from within the project folder, is filling the `compose/medusa/services.d` folder with your services definition YAML files (see documentation above), and then run:

```sh
docker-compose up
```

**NOTE:** In both examples, the host network is used. This means that the containers will bind directly to the host ports and network interface in order to be reachable from attackers. You might want to customize this setup depending on your network infrastructure.

Lastly, you can build from sources if you have Rust installed in your system (it requires `openssl`):

```sh
cargo build 
```

## Shodan Host Clone

You can use `medusa` to create a (best-effort) clone of a device that's indexed on shodan.io. 

In order to do this you'll need an API key:

```sh
export SHODAN_API_KEY=your_api_key_here
```

Then you can clone a host (`38.18.235.213` in this example) with:

```sh
docker run -v $(pwd)/mikrotik:/mikrotik medusa \
  --shodan-api-key $SHODAN_API_KEY \
  --shodan-clone 38.18.235.213 \
  --output /mikrotik
```

This will create the YAML service files inside the `mikrotik` folder. This folder can then be used with:

```sh
docker run \
  -v $(pwd)/mikrotik:/etc/medusa/services.d \
  -v $(pwd)/records:/var/lib/medusa/records \
  --network host \
  medusa
```

## Usage

First you need to create at least one service file. Let's begin by defining a simple SSH honeypot that accepts any combination of user and password:

```sh
mkdir -p /path/to/services.d/
touch /path/to/services.d/example-ssh.yml
```

Open `/path/to/services.d/example-ssh.yml` with your favorite editor and paste these contents:


```yaml
proto: ssh
address: '127.0.0.1:2222'
server_id: 'SSH-2.0-OpenSSH_7.2p2 Ubuntu-4ubuntu2.10'
banner: 'Last login: Mon Sep  5 14:12:09 2022 from 127.0.0.1'
prompt: '# '
timeout: 15
commands:
  - parser: '^exit(\s.+)?$'
    handler: '@exit'
```

In some cases, custom SSH servers do not respect the RFC standard for server id termination characters `\r\n`, but rather use only `\n`. In this case it is possible to use the `server_id_raw` directive and include custom terminators in the string, while of `server_id` is used the default `\r\n` will be used. 

```yaml
proto: ssh
address: '127.0.0.1:2222'
server_id_raw: "SSH-1.99-TECHNICOLOR_SW_2.0\n"
banner: 'Last login: Mon Sep  5 14:12:09 2022 from 127.0.0.1'
prompt: '# '
timeout: 15
commands:
  - parser: '^exit(\s.+)?$'
    handler: '@exit'
```

Now run:

```sh
medusa --services "/path/to/services.d/" --records "/path/to/output/records"
```

This will start a single honeypoint on port 2222 and all the resulting events will be saved as JSON files in the folder indicated by `--records`.

## Commands

The previous example won't do much if somebody tries to execute actual commands. It only captures the `exit` command in order to terminate the session (via the `@exit` special handler). Let's add another command, for instance to parse simple `echo` inputs:

```yaml
proto: ssh
address: '127.0.0.1:2222'
server_id: 'SSH-2.0-OpenSSH_7.2p2 Ubuntu-4ubuntu2.10'
prompt: '# '
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
    handler: "<html><body>hello world</body></html>"
```

### Docker Jail

Another useful handler is `@docker`. As the name suggests it executes any shell command it receives as an argument inside a docker container, therefore we could create a "jailed" `ssh` honeypot by doing:

```yaml
proto: ssh
address: '127.0.0.1:2222'
server_id: 'SSH-2.0-OpenSSH_7.2p2 Ubuntu-4ubuntu2.10'
prompt: '# '
timeout: 15
commands:
  - parser: '^exit(\s.+)?$'
    handler: '@exit'
  - parser: '^(.+)$'
    handler: '@docker jail {$1}'
```

You can create and start a `jail` container with:

```sh
docker container create --name jail busybox tail -f /dev/null # feel free to pick any image
docker start jail
```

This will execute any command that the client is sending on the `jail` container and it will transparently pass the output to the client.

Configuring a realistic docker container is beyond the purpose of this document, you can find useful images [here](https://github.com/plajjan/vrnetlab).

## Protocols

SSH server emulation (with docker jail):

```yaml
proto: ssh
address: '127.0.0.1:2222'
server_id: 'SSH-2.0-OpenSSH_7.2p2 Ubuntu-4ubuntu2.10'
prompt: '# '
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

HTTPS is also supported, you'll need to generate a new RSA key and certificate first:

```sh
 openssl req -newkey rsa:2048 -nodes -keyout medusa-https.key -x509 -days 365 -out medusa-https.crt
```

Then just enable `tls` in your `http` service configuration:

```yaml
proto: http 
address: '127.0.0.1:8181'
tls: true
key: medusa-https.key
certificate: medusa-https.crt
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

And UDP servers as well:

```yaml
proto: udp
address: '0.0.0.0:5353'
banner: |
  dnsmasq-2.73
  Recursion: enabled
  Resolver name: X4200
```

## License

Medusa was made with ♥  by [Simone Margaritelli](https://www.evilsocket.net/) and it's released under the GPL 3 license.