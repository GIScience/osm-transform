FROM rust:latest AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/osm-transform /usr/local/bin/osm-transform
ENTRYPOINT ["/usr/local/bin/osm-transform"]
CMD []
