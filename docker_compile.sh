#!/usr/bin/env sh
docker build -t ors-preprocessor -f Dockerfile .
docker run -i -v /opt/ors/elevation/preprocessor:/elevation -v /opt/ors/osm/preprocessor:/osm ors-preprocessor mo /osm/planet-latest.osm.pbf 
docker container prune -f
docker image rm ors-preprocessor:latest
