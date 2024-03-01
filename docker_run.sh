#!/bin/bash
docker rm -f osm-transform || true
docker build -t osm-transform .
docker run -it -v .:/osm --name osm-transform --user "$(id -u):$(id -g)" osm-transform $@