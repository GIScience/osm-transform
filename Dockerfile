FROM rust:alpine3.20

# Install necessary packages
RUN apk add --no-cache musl-dev pkgconfig openssl-dev

WORKDIR /usr/src/rusty-routes-transformer

COPY . /usr/src/rusty-routes-transformer

RUN cargo build --color=always --package rusty-routes-transformer --bin rusty-routes-transformer --release

RUN cargo install --path .

WORKDIR /osm

ENTRYPOINT ["/usr/local/cargo/bin/rusty-routes-transformer"]