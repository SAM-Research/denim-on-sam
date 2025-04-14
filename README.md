# Denim on SAM

[![Rust](https://github.com/SAM-Research/denim-on-sam/actions/workflows/rust.yml/badge.svg)](https://github.com/SAM-Research/denim-on-sam/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/SAM-Research/denim-on-sam/graph/badge.svg?token=LeCEZUDsc9)](https://codecov.io/gh/SAM-Research/denim-on-sam)

# Usage

```
$ denim-sam-proxy --help
Usage: denim-sam-proxy [OPTIONS]

Options:
  -s, --sam-address <sam_address>
          Address to run sam server on [default: 127.0.0.1:8080]
  -p, --proxy-address <proxy_address>
          Address to run proxy on [default: 127.0.0.1:8081]
  -q, --deniable-ratio <deniable_ratio>
          Deniable to regular payload ratio (q) [default: 1]
  -b, --buffer-size <buffer_size>
          How many messages can be in a buffer channel before blocking behaviour [default: 10]
  -c, --config <config>
          JSON Config path
  -h, --help
          Print help
```

## JSON Configuration

You can configure the proxy with TLS and mTLS for communication with SAM Server and with denim clients by doing:

```sh
denim-sam-proxy --config ./config.json
```

where the looks like this:

```jsonc
{
  "samAddress": "127.0.0.1:8080", // Address to sam (optional)
  "denimProxyAddress": "127.0.0.1:8081", // Address to run DenIM Proxy on (optional)
  "deniableRatio": 1.0, // Deniable ratio (q) (optional)
  "channelBufferSize": 10, // Internal message communication, might affect performance of proxy (optional)

  "tls": {
    // Tls config (optional)
    "caCertPath": "./e2e/cert/rootCA.crt", // Certificate Authority certificate path
    "proxyCertPath": "./e2e/cert/proxy.crt", // Cerficate for proxy
    "proxyKeyPath": "./e2e/cert/proxy.key", // Certificate Key for proxy
    "proxyMtls": false, // set to true if denim clients need to communicate with the proxy over mTLS instead of TLS
    "proxyClient": {
      // mTLS Proxy Connection to SAM Server (optional)
      "certPath": "./e2e/cert/client.crt", // Proxys Client Cert
      "keyPath": "./e2e/cert/client.key" // Proxys Client Cert
    }
  }
}
```

# Docker

Building the `denim-sam-proxy` docker image:

```sh
docker build -t denim-sam-proxy .
```

# SAM Dependencies

to update sam dependencies just do:

```sh
cargo update -p sam-server
```

you might need to change `sam-server` to either one of the other sam projects

# End-To-End tests

In order to run the end-to-end tests, you need to generate certificates.

1. Go into `scripts`
2. Generate certificates by running the following

```zsh
./generate_cert.sh ../e2e/cert
```
