FROM rust:slim-bookworm as builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
COPY src src
COPY examples examples

RUN cargo build --release --example basic_consumer

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /

COPY --from=builder /usr/src/app/target/release/examples/basic_consumer /opt/sqs-consumer/bin/sqs-consumer

ENTRYPOINT ["/opt/sqs-consumer/bin/sqs-consumer"]