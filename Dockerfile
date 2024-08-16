FROM rust:1.80-alpine3.20

# Install necessary packages
RUN apk add --no-cache musl-dev pkgconfig openssl-dev lld && \
    mkdir -p /usr/src/rusty-routes-transformer && \
    mkdir -p /usr/local/cargo/bin

WORKDIR /usr/src/rusty-routes-transformer

COPY . .

ENV MALLOC_CONF="thp:always,metadata_thp:always"
# Set TARGET and RUSTFLAGS based on the system architecture arm / armhf / aarch64 / x86_64
RUN if [ "$(uname -m)" = "arm64" ]; then \
        TARGET=aarch64-unknown-linux-musl \
        export RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld -C target-feature=+v8,+fp+simd"; \
    elif [ "$(uname -m)" = "arm" ]; then \
        TARGET=armv7-unknown-linux-musleabi \
        export RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld -C target-feature=+v7,+vfp4,+neon"; \
    elif [ "$(uname -m)" = "armhf" ]; then \
        TARGET=armv7-unknown-linux-musleabihf \
        export RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld -C target-feature=+v7,+vfp4,+neon"; \
    elif [ "$(uname -m)" = "x86_64" ]; then \
        TARGET=x86_64-unknown-linux-musl \
        export RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld -C target-feature=+avx2,+fma"; \
    else \
        echo "Unsupported architecture"; \
        exit 1; \
    fi && \
    echo "Building for $TARGET" && \
    cargo build --release --bin rusty-routes-transformer --target $TARGET

RUN cp target/$TARGET/release/rusty-routes-transformer /usr/local/cargo/bin
RUN apk del musl-dev pkgconfig openssl-dev lld

WORKDIR /osm

ENTRYPOINT ["/usr/local/cargo/bin/rusty-routes-transformer"]