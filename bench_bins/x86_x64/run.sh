#!/usr/bin/env bash

# This script is used to run the benchmark binaries in the bench_bins directory.

curl -L -O -C - https://download.geofabrik.de/europe/germany/baden-wuerttemberg/karlsruhe-regbez-latest.osm.pbf
curl -L -O -C - https://download.geofabrik.de/europe/germany/baden-wuerttemberg-latest.osm.pbf
curl -L -O -C - https://download.geofabrik.de/europe/germany-latest.osm.pbf


hyperfine --runs 1 -L PBF karlsruhe-regbez-latest \
'./rusty-routes-transformer-vector-handlers --input-pbf {PBF}.osm.pbf --output-pbf {PBF}.ors.pbf' \
'./rusty-routes-transformer-vector-handlers-cargo-opt --input-pbf {PBF}.osm.pbf --output-pbf {PBF}.ors.pbf' \
'./rusty-routes-transformer-vector-handlers-cargo-opt-build-perf --input-pbf {PBF}.osm.pbf --output-pbf {PBF}.ors.pbf' \
'./rusty-routes-transformer-add-dockerfile-cargo-opt-build-perf --input-pbf {PBF}.osm.pbf --output-pbf {PBF}.ors.pbf' \
'./ors-preprocessor {PBF}.osm.pbf'

