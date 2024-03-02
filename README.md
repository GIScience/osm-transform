# `osm-transform`

Tool for reduction of OSM data and processing cost for elevation data extraction during graph building.
Removes metadata, unused elements and tags, and adds `ele` tags to all retained nodes.

## Usage

There are several possibilities to build and run osm-transform.

### Run with Docker

To run osm-transform with Docker, first, make sure that [Docker](https://www.docker.com/) is installed on the machine and clone this repository. From the root directory, you can run:

```shell
./docker_run.sh -p planet-latest.osm.pbf
```

This builds the Docker image and runs the preprocessor as a container, passing the provided OSM file as source file. The script mapping the current working directory to the working directory within the container. To change this behavior, you could also pass the option `-w`.

```shell
./docker_run.sh -w /path/to/osm-transform-working-dir -p planet-latest.osm.pbf
```

Note that in this case the path to the OSM file needs to be relative to the working directory you passed.

You can build and run the Docker container manually: 

```shell
docker build -t osm-transform .
docker run -it -v .:/osm osm-transform -p planet-latest.osm.pbf
```

The `-v` option is mapping the current working directory to the working directory within the container. `-p planet-latest.osm.pbf` points to the OSM file to use.

### Build from the command line

To build osm-transform on the command line we recommend you use cmake/ninja/g++.

Prerequisites for installing osm-transform at the command line are the following libraries:

- [libgdal](https://gdal.org/)
- [libproj](https://proj.org/)
- [libosmium](https://osmcode.org/libosmium/)
- [boost](https://www.boost.org/)

From the project root directory, run

```shell
# on ubuntu
sudo apt install g++ cmake ninja-build libgdal-dev libproj-dev libosmium-dev libboost-all-dev
cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_MAKE_PROGRAM=/usr/bin/ninja -G Ninja -B ./cmake-build
cmake --build ./cmake-build --target osm-transform
cp ./cmake-build/osm-transform .
```

You can then use the tool by running

```shell
./osm-transform -p planet-latest.osm.pbf
```

### Usage details

The minimal required argument that needs to be provided is the OSM data file to process.

```
Generic options:
  -v [ --version ]                      print version string
  -h [ --help ]                         produce help message

Configuration:
  -e [ --skip ]                         skip elevation data merge
  -i [ --interpolate ]                  interpolate intermediate nodes
  -p [ --osm-pbf ] arg                  Absolute file path to osm pbf file to process
  -T [ --remove_tag ] arg               (=(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia) regex to match removable tags
  -F [ --geo_tiff_folders ] arg         (=tiffs, srtmdata, gmteddata) absolute paths to Geotiff folders 
  -S [ --cache_limit ] arg              (=1073741824) maximum memory used to store tiles in cache
  -t [ --threshold ] arg                (=0.5) only used in combination with interpolation, threshold for elevation
  -f [ --config-file ] arg              absolute file path to config file to use
  -d [ --debug_mode ]                   debug_mode
```

[//]: # (TODO update all below)

The `osm-transform.cfg` file is used to set up the tool. The default configuration is as follows:

```
# number of elevation tiles to keep open in memory simultaneously
cache_limit = 10;

# regex for detecting tags that can safely be removed
remove_tag = "(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia";

# activate debug output
#debug_mode = true;
```

- The `cache_limit` determines how many tile GTIFs are kept open in memory during the processing.
  A too high number might result in paging overhead degrading performance.
- All tags matching the `remove_tag` regex are stripped from the data.
- The max IDs settings must be adjusted when OSM data grows.
  The configured number must be higher than the highest ID number for each category of elements in the source file.
  Note that memory consumption grows proportionally. The settings above are enough for current OSM planet file.
- The `debug_mode` flag activates some verbose diagnostic output.
- The `debug_no_filter` flag deactivates filtering of elements, so that only metadata and tags are reduced.
- The `debug_no_tag_filter` flag deactivates filtering of tags, so that all tags are retained.

### Elevation data

For the elevation data merge, also download the elevations tiles for CGIAR SRTM data (
see [srtm.csi.cgiar.org](http://srtm.csi.cgiar.org/)) and
GMTED2010 data (as a fallback option,
see [GMTED2010](https://www.usgs.gov/land-resources/eros/coastal-changes-and-impacts/gmted2010)).
A python script is provided that can be used to download (and, in the case of CGIAR data, unpack) these resources.
Be aware those require approx. 63.1 Gb and 26.3 Gb drive space respectively.

You need [python](https://www.python.org/downloads/) and [poetry](https://python-poetry.org/docs/) installed on your
system.
In the project directory, run

```
poetry install 
poetry run osm-transform srtm-download
poetry run osm-transform gmted-download
```

This will create directories `srtmdata` and `gmteddata` respectively and download the geotiff tiles.

## Functionality details

The tool processes OSM data in two steps.

During the first pass, it filters way and relation elements and records the IDs and those of all referenced nodes where
the elements are valid and used by ORS graphs.
This process removes OSM ways representing objects unrelated to the road graph, with tags like "building", "landuse",
"boundary", "natural" and a few others.

During the second pass, a new PBF file `[file].ors.pbf` is written, containing only the relevant elements for the ORS
graphs.
From the retained elements, all tags matching the `remove_tag` regular expression (see configuration file example above)
as well as irrelevant metadata (version, user_id, timestamp etc.) are stripped.

Before the first pass, the tool needs to allocate the required memory to store the flags for the valid data sets in
the source file. The max IDs numbers set in the config file need to be larger than the highest ID in the source file.
You can use the option `-m` to let the tool determine the required memory automatically or option `-c` to only do the
memory check to adjust the config values.

For each node, the preprocessor determines the elevation from the CGIAR data (if no data is available, falls back to
GMTED data).
You can skip this step by setting the `-e` option. Any `ele` tags already present in the OSM data are overwritten,
since this is current ORS (and GH) behavior.
You can pass the `-o` option to have the preprocessor retain the `ele` tag values where present in the OSM data.
The elevation value defaults to 0 where it could not be determined (outside tiles coverage or invalid data in both data
sets), since this is GH behavior.

Overall, a ca. 70% file size reduction is achieved. Skipping the elevation data merge increases this only by a few
percent, and since the elevation data retrieval during graph building (which can be skipped if the preprocessor already
merged the elevation data) has a significant impact on memory consumption during graph building,
the default behavior is to include this operation.

Example console output while processing OSM planet file:

```
Max IDs from config: Node 7300000000 Way 800000000 Relation 20000000
Allocating memory: 870.23 Mb nodes, 95.37 Mb ways, 2.38 Mb relations

Processing first pass: validate ways & relations...
[======================================================================] 100% %
valid nodes: 1551699772 (5385098645), valid ways: 165335973 (596956981), valid relations: 3006399 (6985641)
Processed in 1043.549 s

Processing second pass: rebuild data...
Progress: 5989041267 / 5989041267 (100.00 %)
Processed in 10288.611 s

Original:          49796963050 b
Reduced:           14914077359 b
Reduction:         34882885691 b (= 70.05 %)
Elevation:                0.04 % failed (660459)
```

A log file is written containing all coordinates where no elevation value could be determined.
The number of cases in the above example is normal (paths on Antarctica etc.).

### Developers: CLion (Nova) cmake setup

- Open the root folder of the cloned repository (osm-transform) with CLion
- Choose reload CMake project automatically if CMakeLists.txt changes
- Go to preferences->Build,Execution,Deployment->Toolchain
- Set compiler for default toolchain to g++ (e.g. `usr/bin/g++`)
- Choose `Vcpkg` from bottom menu bar (or double tap `shift` and search `vcpkg`)
- Add vcpkg if no entry exists
- Choose defaults and check add to existing CMake profiles
- Install `libosmium`, `boost-program-options` and `gdal`(if not installed on system). This should install the correct
  binaries for your OS.
- Edit the autogenerated run configuration:
    - Program arguments: `-m path-to-your.osm.pbf`
    - Working directory: `absolute-path-to/osm-transform`
- Build, Run and Debug should work now.

If not:

- Go to preferences->Build,Execution,Deployment->Cmake
- Make sure `CMake options` contains `-DCMAKE_TOOLCHAIN_FILE=` with value like
  `/Users/your-user/.vcpkg-clion/vcpkg/scripts/buildsystems/vcpkg.cmake`

## Limitations & future development

This tool is under development and still experimental, though it has been successfully tested on several OSM subsets
(Germany, DACH). Use at own risk.

- Baden-Württemberg (from [geofabrik](http://download.geofabrik.de/europe/germany/baden-wuerttemberg.html)) runs in
  about 90 seconds on my ThinkPad, the OSM planet file in ca. 3 hours.
- The caching of elevation tile files might influence this, could be further optimized.
- The tags removal currently just removes a few typical ignorable tags, but a thorough statistical analysis of tags in
  OSM to identify the most frequently used tags could yield better results.
- The downloading of elevation geotiff files might be integrated into the tool for convenience.
  It was just quicker to write a python script...

