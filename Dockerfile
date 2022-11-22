FROM rust:1-buster as builder
WORKDIR /usr/src/mail-to-telegram
COPY . .
RUN cargo build --release

FROM debian:buster-slim
COPY --from=builder /usr/src/mail-to-telegram/target/release/mail-to-telegram /usr/local/bin/mtl
ENTRYPOINT ["/usr/local/bin/mtl"]