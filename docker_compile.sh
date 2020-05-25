#!/usr/bin/env sh
docker build -t ors-preprocessor -f Dockerfile .
docker run -it -v /opt/ors/elevation/preprocessor:/elevation -v /opt/ors/osm/preprocessor:/osm --name ors-preprocessor ors-preprocessor -m -o /osm/planet-latest.osm.pbf 
docker rm ors-preprocessor