FROM debian:bookworm-slim

RUN set -eux; \
  apt-get update; \
  apt-get install -y --no-install-recommends \
    g++ \
    cmake \
    ninja-build \
    libgdal-dev \
    libproj-dev \
    libosmium-dev \
    libboost-regex-dev \
    libboost-program-options-dev \
    libboost-test-dev \
    ; \
    \
    rm -rf /var/lib/apt/lists/*;
 
COPY . /osm-transform/
 
RUN set -eux; \
   cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_MAKE_PROGRAM=/usr/bin/ninja -G Ninja -S /osm-transform -B /osm-transform/cmake-build ; \
   cmake --build /osm-transform/cmake-build --target osm-transform ; \
   cmake --install /osm-transform/cmake-build

WORKDIR /osm
ENTRYPOINT ["/usr/local/bin/osm-transform"]:
