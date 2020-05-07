FROM alpine:3.11

WORKDIR /build

RUN apk add --no-cache --virtual .build-deps \
    git \
    make \
    cmake \
    g++ \
    boost-dev \
    gdal-dev \
    libconfig-dev \
    bzip2-dev \
    expat-dev \
    geos-dev \
    proj-dev \
    sparsehash \
    zlib-dev && \
  cd /build && \
  git clone https://github.com/mapbox/protozero.git && \
    cd protozero && \
    mkdir build && \
    cd build && \
    cmake .. && \
    make && \
    make install && \
  cd /build && \
  git clone https://github.com/osmcode/libosmium.git && \
    cd libosmium && \
    mkdir build && \
    cd build && \
    cmake -DCMAKE_BUILD_TYPE=MinSizeRel -DBUILD_EXAMPLES=OFF .. && \
    make && \
    make install

ENTRYPOINT echo "Compiling ors-preprocessor..." && \
  cd /src && \
  g++ ors-preprocessor.cpp -o ors-preprocessor --std=c++11 -m64 -lpthread -lz -lexpat -lbz2 -lconfig++ -lgdal -lboost_regex -lboost_system -O3 && \
  echo "done." && \
  echo "Performing cleanup."
