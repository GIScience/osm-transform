# rusty-routes-transformer

Our plan:

* read and write pbf
* filter geometries not relevant for routing
* filter user tags not relevant for routing
* match elevation data from geotiffs
    * split edges at tiff resolution
* auto-download of elevation data
    * SRTM cigar
    * GMTED
* matching of geometry data like country borders, time zones etc. to enrich output
* cli with options
* config file

## Usage native

```bash
cargo run --release -- -i input.osm.pbf -o output.osm.pbf
```

## Usage docker

```bash
./docker_run.sh --input-pbf input.osm.pbf --output-pbf output.ors.pbf
```