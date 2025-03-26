# Denim on SAM

[![Rust](https://github.com/SAM-Research/denim-on-sam/actions/workflows/rust.yml/badge.svg)](https://github.com/SAM-Research/denim-on-sam/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/SAM-Research/denim-on-sam/graph/badge.svg?token=LeCEZUDsc9)](https://codecov.io/gh/SAM-Research/denim-on-sam)

to update sam dependencies just do:

```sh
cargo update -p sam-server
```

## TLS

You can configure the proxy with TLS and mTLS for communication with SAM Server and with denim clients by doing:

```sh
cargo run --bin denim-sam-proxy -- --tls-config ./config.json
```

where the looks like this:

```jsonc
{
  "caCertPath": "./rootCA.crt",
  "proxyCertPath": "./proxy.crt",
  "proxyKeyPath": "./proxy.key",
  "proxyMtls": false, // require denim clients to have mTLS
  "proxyClient": {
    // this is for the internal proxy client for communication to SAM server
    "certPath": "./client.crt",
    "keyPath": "./client.key"
  }
}
```

# End-To-End tests

In order to run the end-to-end tests, you need to generate certificates.

1. Go into `scripts`
2. Generate certificates by running the following

```zsh
./generate_cert.sh ../e2e/cert
```
