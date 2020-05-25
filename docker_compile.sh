#!/bin/bash

while getopts o:e: option
do
case "${option}"
in
o) OSM_PATH=${OPTARG};;
e) ELEVATION_PATH=${OPTARG};;
esac
done

if [ -z ${OSM_PATH+x} ]; then OSM_PATH=/opt/ors/osm; fi
if [ -z ${ELEVATION_PATH+x} ]; then ELEVATION_PATH=/opt/ors/elevation; fi

docker build -t ors-preprocessor -f Dockerfile .
docker run -it -v $ELEVATION_PATH:/elevation -v $OSM_PATH:/osm --name ors-preprocessor ors-preprocessor -m -o /osm/planet-latest.osm.pbf 
docker rm ors-preprocessor