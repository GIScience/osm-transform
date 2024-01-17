FROM ubuntu:jammy
# For Mac with amd chip use
#FROM --platform=linux/amd64 ubuntu:jammy

RUN apt-get -qq update && apt-get -y -qq install apt-utils

RUN DEBIAN_FRONTEND=noninteractive apt-get -y -qq install \
	libboost-all-dev \
	libgdal-dev \
	libosmium-dev \
	libconfig++-dev \
	g++ 

COPY . /src
WORKDIR /src
RUN g++ *.cpp -o osm-transform --std=c++20 -m64 -lpthread -lz -lexpat -lbz2 -lconfig++ -I/usr/include/gdal -lgdal -lboost_regex -lboost_system -O3

RUN mkdir /osm
WORKDIR /osm

ENTRYPOINT ["/src/osm-transform", "options", "osm"]

