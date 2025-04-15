FROM rust:1.86-slim AS builder
ENV SQLX_OFFLINE=true
WORKDIR /denim-on-sam
COPY . .

RUN apt update 
RUN apt install -y protobuf-compiler

RUN cargo build --bin denim-sam-proxy --release

LABEL org.opencontainers.image.source=https://github.com/SAM-Research/denim-on-sam
LABEL org.opencontainers.image.description="Denim SAM Proxy image"
LABEL org.opencontainers.image.licenses=MIT

FROM debian:bookworm-slim

COPY --from=builder /denim-on-sam/target/release/denim-sam-proxy /denim-sam-proxy

ENV PORT=8081

ENTRYPOINT ["/denim-sam-proxy"]
EXPOSE $PORT
