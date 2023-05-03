#!/bin/bash

while getopts o:e:f:p: option
do
case "${option}"
in
o) OSM_PATH=${OPTARG};;
e) ELEVATION_PATH=${OPTARG};;
f) OSM_FILE=${OPTARG};;
p) ROOT_PATH=${OPTARG};;
esac
done

if [ -z ${OSM_PATH+x} ]; then OSM_PATH=/opt/ors/osm; fi
if [ -z ${ELEVATION_PATH+x} ]; then ELEVATION_PATH=/opt/ors/elevation; fi
if [ -z ${OSM_FILE+x} ]; then OSM_FILE=planet-latest.osm.pbf; fi
if [ -z ${ROOT_PATH+x} ]; then ROOT_PATH=/opt/ors/osm-transform; fi

cd $ROOT_PATH
docker rm osm-transform_$OSM_FILE
docker build -t osm-transform -f Dockerfile .
docker run -it -v $ELEVATION_PATH:/elevation -v $OSM_PATH:/osm --name osm-transform_$OSM_FILE osm-transform -m -o /osm/$OSM_FILE