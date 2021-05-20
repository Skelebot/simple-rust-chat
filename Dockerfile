#syntax=docker/dockerfile:1
FROM rust:latest
COPY . .
RUN cargo build --release --bin server
ENTRYPOINT ["target/release/server"]

