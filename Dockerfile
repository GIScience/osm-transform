FROM ubuntu:jammy

RUN apt-get -qq update && apt-get -y -qq install apt-utils

RUN DEBIAN_FRONTEND=noninteractive apt-get -y -qq install \
	libboost-all-dev \
	libgdal-dev \
	libosmium-dev \
	libconfig++-dev \
	g++ 

COPY . /src
WORKDIR /src
RUN g++ ors-preprocessor.cpp -o ors-preprocessor --std=c++11 -m64 -lpthread -lz -lexpat -lbz2 -lconfig++ -I/usr/include/gdal -lgdal -lboost_regex -lboost_system -O3

RUN mkdir /osm
WORKDIR /osm

ENTRYPOINT ["/src/ors-preprocessor", "options", "osm"]

