# rusty-routes-transformer

A command line tool to preprocess OpenStreetMap data for routing purposes. 
The tool reads an OpenStreetMap pbf file, filters data that is not relevant for routing (e.g. buildings) and produces a much smaller pbf file.
At the same time, the tool enriches the data with information about the country the data is in and optionally with elevation data.

The output pbf file can be used e.g. for openrouteservice graph building.

## Run tests
Run all tests:
```shell
cargo test
```

Integration tests:
```shell
cargo test --test integration_test
```

## Run locally
```shell
cargo clean
cargo run -- <options>
```

To get help about the command line options, run:
```shell
cargo run -- -h
```

## Build locally
```shell
cargo clean
cargo build
```

Or a optimized, faster version (build is slower):

```shell
cargo clean
cargo build --release
```

The executable binaries created by `cargo build` or `cargo build --release` are located in the `target/debug` or `target/release` directory respectively.

You can run the executable with the same options as described above for running locally:
```shell
./target/debug/rusty-routes-transformer <options>
./target/release/rusty-routes-transformer <options>
```


## Docker

### Build docker image

Run docker build and tag the image with e.g. `rrt:latest`:
```shell
docker rmi rrt:latest # if you have an old image
docker build -t rrt:latest .
```

### Run docker image

When running the docker image, you can add all command line options for the rust application behind the docker image name. 
E.g. you can get help by running the following command:

```shell
docker run --rm rrt:latest -h
```

In all other use cases than getting help, you want to process an input file and potentially enrich the data with 
information from other files. Therefore you need to mount the directories containing the input files to the docker.
The following command mounts the directories `~/data/osm`, `~/data/countries` and `~/data/elevation` to the docker
directories `/app/osm`, `/app/countries` and `/app/elevation` respectively.
You can mount whichever directories you want, but you need to adjust the paths in the command according to the 
paths in the container:

```shell
docker run --rm \
    -v ~/data/osm:/app/osm \
    -v ~/data/countries:/app/countries \
    -v ~/data/elevation:/app/elevation \
    -v .:/app/out \
    rrt:latest \
    -i /app/osm/heidelberg.test.pbf \
    -o /app/out/heidelberg.rrt.pbf \
    -c /app/countries/world_borders_idx_0_40 \
    -e '/app/elevation/*/*.tif' \
    -vvv
```

## Roadmap

* performance and memory usage improvements
* enable custom filter options
* support for more custom enrichment source files, e.g.:
  * other csv area with other information than countries, e.g. time zones 
  * other geotiff files for raster based enrichment
* make it possible to only build area indexes without processing a pbf file