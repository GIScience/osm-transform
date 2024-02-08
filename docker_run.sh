#!/bin/bash

while getopts ":w:p:" option;
do
# shellcheck disable=SC2220
case "${option}"
in
w) WORKDIR=${OPTARG};;
p) OSM_FILE=${OPTARG};;
esac
done

if [ -z ${WORKDIR+x} ]; then WORKDIR=.; fi
if [ -z ${OSM_FILE+x} ]; then OSM_FILE=planet-latest.osm.pbf; fi

docker rm -f osm-transform || true
docker build -t osm-transform .
docker run -it -v $WORKDIR:/osm --name osm-transform --user "$(id -u):$(id -g)" osm-transform -p /osm/$OSM_FILE