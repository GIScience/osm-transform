FROM ubuntu:jammy as build
# For Mac with amd chip use
#FROM --platform=linux/amd64 ubuntu:jammy

RUN apt-get -qq update && apt-get -y -qq install apt-utils

RUN DEBIAN_FRONTEND=noninteractive apt-get -qq update && apt-get -qq install \
	g++ \
    cmake \
    ninja-build \
	libgdal-dev \
	libproj-dev \
	libosmium-dev \
	libboost-all-dev

COPY ./test /src/test
COPY *.h *.cpp CMakeLists.txt /src/
WORKDIR /src

RUN cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_MAKE_PROGRAM=/usr/bin/ninja -G Ninja -S /src -B /src/cmake-build
RUN cmake --build /src/cmake-build --target osm-transform

RUN mkdir /osm
WORKDIR /osm

ENTRYPOINT ["/src/cmake-build/osm-transform"]
