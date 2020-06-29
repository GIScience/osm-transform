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
if [ -z ${ROOT_PATH+x} ]; then ROOT_PATH=/opt/ors/ors-preprocessor; fi

cd $ROOT_PATH
docker rm ors-preprocessor_$OSM_FILE
docker build -t ors-preprocessor -f Dockerfile .
docker run -it -v $ELEVATION_PATH:/elevation -v $OSM_PATH:/osm --name ors-preprocessor_$OSM_FILE ors-preprocessor -m -o /osm/$OSM_FILE 