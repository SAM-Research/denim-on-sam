FROM messense/rust-musl-cross:x86_64-musl AS builder
ENV SQLX_OFFLINE=true
WORKDIR /denim-on-sam
COPY . .

RUN apt update 
RUN apt install -y protobuf-compiler
RUN apt update && apt install -y libssl-dev pkg-config

RUN cargo build --bin denim-sam-proxy --release --target x86_64-unknown-linux-musl

LABEL org.opencontainers.image.source=https://github.com/SAM-Research/denim-on-sam
LABEL org.opencontainers.image.description="Denim SAM Proxy image"
LABEL org.opencontainers.image.licenses=MIT


RUN ls -l /denim-on-sam/target/x86_64-unknown-linux-musl/release/

FROM scratch
COPY --from=builder /denim-on-sam/target/x86_64-unknown-linux-musl/release/denim-sam-proxy /denim-sam-proxy

ENV PORT=8081

ENTRYPOINT ["/denim-sam-proxy"]
EXPOSE $PORT