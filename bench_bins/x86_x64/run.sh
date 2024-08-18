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

# Check if the planet.osm.pbf file exists
if [ ! -f planet-latest.osm.pbf ]; then
  echo "Downloading planet-latest.osm.pbf"
  # curl with output to planet.osm.pbf and continue downloading if the download is interrupted
  curl -L -O -C - https://ftp.fau.de/osm-planet/pbf/planet-latest.osm.pbf
fi

# Set environment variables. Important for the benchmark results.
# shellcheck disable=SC2034
RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld -C target-feature=+avx2,+fma"
# shellcheck disable=SC2034
MALLOC_CONF="thp:always,metadata_thp:always"

echo "Running the benchmark binaries..."
# Create a directory to store the benchmark results
mkdir -p bench_results
# Get date and time
now=$(date +"%Y-%m-%d_%H-%M-%S")

#json_file="bench_results/bench_results_karlsruhe_baden_wuerttemberg_${now}.json"
#markdown_file="bench_results/bench_results_karlsruhe_baden_wuerttemberg_${now}.md"
#hyperfine --export-json "${json_file}" --export-markdown "${markdown_file}" \
#--runs 3 -L PBF karlsruhe-regbez-latest,baden-wuerttemberg-latest \
#'./rusty-routes-transformer-add-dockerfile-cargo-opt-build-perf --input-pbf {PBF}.osm.pbf --output-pbf {PBF}.ors.pbf' \
#'./osm-transform -p {PBF}.osm.pbf' \
#'./ors-preprocessor {PBF}.osm.pbf'

json_file="bench_results/bench_results_germany_planet${now}.json"
markdown_file="bench_results/bench_results_germany_planet${now}.md"
hyperfine --export-json "${json_file}" --export-markdown "${markdown_file}" \
--runs 1 -L PBF germany-latest,planet-latest \
'./rusty-routes-transformer-add-dockerfile-cargo-opt-build-perf --input-pbf {PBF}.osm.pbf --output-pbf {PBF}.ors.pbf' \
'./osm-transform -p {PBF}.osm.pbf' \
'./ors-preprocessor {PBF}.osm.pbf'
