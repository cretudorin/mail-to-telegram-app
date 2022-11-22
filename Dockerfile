FROM rust:1-buster as builder
WORKDIR /usr/src/mail-to-telegram
COPY . .
RUN cargo build --release

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl-dev openssl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/mail-to-telegram/target/release/mail-to-telegram /usr/local/bin/mtl
ENTRYPOINT ["/usr/local/bin/mtl"]