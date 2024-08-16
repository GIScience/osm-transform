FROM rust:1.80-alpine3.20

# Install necessary packages
RUN apk add --no-cache musl-dev pkgconfig openssl-dev lld && \
    mkdir -p /usr/src/rusty-routes-transformer && \
    mkdir -p /usr/local/cargo/bin

WORKDIR /usr/src/rusty-routes-transformer

COPY . .

ENV TARGET=x86_64-unknown-linux-musl
ENV RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld -C target-feature=+avx2,+fma"
ENV MALLOC_CONF="thp:always,metadata_thp:always"
# Set TARGET and RUSTFLAGS based on the system architecture arm / armhf / aarch64 / x86_64
RUN cargo build --release --bin rusty-routes-transformer --target $TARGET && \
    cp target/$TARGET/release/rusty-routes-transformer /usr/local/cargo/bin
RUN apk del musl-dev pkgconfig openssl-dev lld

WORKDIR /osm

ENTRYPOINT ["/usr/local/cargo/bin/rusty-routes-transformer"]