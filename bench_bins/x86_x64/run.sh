#!/usr/bin/env bash

# This script is used to run the benchmark binaries in the bench_bins directory.
echo "Downloading PBF files..."
# check if karlsruhe-regbez-latest.osm.pbf exists
if [ ! -f karlsruhe-regbez-latest.osm.pbf ]; then
  echo "Downloading karlsruhe-regbez-latest.osm.pbf"
  curl -L -O -C - https://download.geofabrik.de/europe/germany/baden-wuerttemberg/karlsruhe-regbez-latest.osm.pbf
fi
# check if baden-wuerttemberg-latest.osm.pbf exists
if [ ! -f baden-wuerttemberg-latest.osm.pbf ]; then
  echo "Downloading baden-wuerttemberg-latest.osm.pbf"
  curl -L -O -C - https://download.geofabrik.de/europe/germany/baden-wuerttemberg-latest.osm.pbf
fi
# check if germany-latest.osm.pbf exists
if [ ! -f germany-latest.osm.pbf ]; then
  echo "Downloading germany-latest.osm.pbf"
  curl -L -O -C - https://download.geofabrik.de/europe/germany-latest.osm.pbf
fi

# Set environment variables. Important for the benchmark results.
# shellcheck disable=SC2034
RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld -C target-feature=+avx2,+fma"
# shellcheck disable=SC2034
MALLOC_CONF="thp:always,metadata_thp:always"

echo "Running the benchmark binaries..."
hyperfine --runs 3 -L PBF karlsruhe-regbez-latest,baden-wuerttemberg-latest,germany-latest \
'./rusty-routes-transformer-vector-handlers --input-pbf {PBF}.osm.pbf --output-pbf {PBF}.ors.pbf' \
'./rusty-routes-transformer-vector-handlers-cargo-opt --input-pbf {PBF}.osm.pbf --output-pbf {PBF}.ors.pbf' \
'./rusty-routes-transformer-vector-handlers-cargo-opt-build-perf --input-pbf {PBF}.osm.pbf --output-pbf {PBF}.ors.pbf' \
'./rusty-routes-transformer-add-dockerfile-cargo-opt-build-perf --input-pbf {PBF}.osm.pbf --output-pbf {PBF}.ors.pbf' \
'./ors-preprocessor {PBF}.osm.pbf'
