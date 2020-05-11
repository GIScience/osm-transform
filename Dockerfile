FROM alpine:3.11

RUN apk add --quiet --no-cache --virtual .build-deps \
	boost-dev \
	bzip2-dev \
	cmake \
	expat-dev \
	g++ \
	gdal-dev \
	geos-dev \
	git \
	libconfig-dev \
	make \
	proj-dev \
	sparsehash \
	zlib-dev

RUN mkdir /build
WORKDIR /build

RUN git clone https://github.com/mapbox/protozero.git
RUN mkdir /build/protozero/build
WORKDIR /build/protozero/build
RUN cmake .. && make && make install

WORKDIR /build
RUN git clone https://github.com/osmcode/libosmium.git
RUN mkdir /build/libosmium/build
WORKDIR /build/libosmium/build
RUN cmake -DCMAKE_BUILD_TYPE=MinSizeRel -DBUILD_EXAMPLES=OFF .. && make && make install

COPY . /src
WORKDIR /src
RUN g++ ors-preprocessor.cpp -o ors-preprocessor --std=c++11 -m64 -lpthread -lz -lexpat -lbz2 -lconfig++ -lgdal -lboost_regex -lboost_system -O3

RUN mkdir /elevation
RUN mkdir /osm

WORKDIR /elevation
#ENTRYPOINT ["/bin/bash"]
ENTRYPOINT ["/src/ors-preprocessor" "options" "osm"]

