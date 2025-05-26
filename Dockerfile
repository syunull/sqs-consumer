FROM rust:slim-bookworm as builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
COPY src src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /

COPY --from=builder /usr/src/app/target/release/main /opt/sqs-consumer/bin/sqs-consumer

ENTRYPOINT ["/opt/sqs-consumer/bin/sqs-consumer"]
