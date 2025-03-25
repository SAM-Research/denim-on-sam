# Denim on SAM

[![Rust](https://github.com/SAM-Research/denim-on-sam/actions/workflows/rust.yml/badge.svg)](https://github.com/SAM-Research/denim-on-sam/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/SAM-Research/denim-on-sam/graph/badge.svg?token=LeCEZUDsc9)](https://codecov.io/gh/SAM-Research/denim-on-sam)

to update sam dependencies just do:

```sh
cargo update -p sam-server
```

# End-To-End tests

In order to run the end-to-end tests, you need to generate certificates.

1. Go into `scripts`
2. Generate certificates by running the following

```zsh
./generate_cert.sh ../e2e/cert
```
