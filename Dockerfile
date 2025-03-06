FROM rust:latest AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/rusty-routes-transformer /usr/local/bin/rusty-routes-transformer
ENTRYPOINT ["/usr/local/bin/rusty-routes-transformer"]
CMD []
