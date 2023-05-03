# `osm-transform`
Tool for reduction of OSM data and processing cost for elevation data extraction during graph building. Removes metadata, unused elements and tags, and adds `ele` tags to all retained nodes.

## Installation
The simplest way of installing the proprocessor is via Docker. First, make sure that Docker is installed on the machine, clone this repository and then build the Docker image. 

```
sudo docker build -t ors-preprocessor .
```

Once built, you can run the preprocessor as a container with the following command:

```
sudo docker run -i -v /[your local absolute path to the dir of the]/ors-preprocessor:/osm ors-preprocessor -m -o planet-latest.osm.pbf 
```

where the `-v` option is the mapping to the folder containing the required files: `cgiar_srtm` and `cgiar_geotiff` folders, the osm data, and the `ors-preprocessor.cfg` file. `-o` is the string reperesenting the script options (see below) and the last item (`planet-latest.osm.pbf`) is the osm file to use. 


Alternatively, you can `make` (Makefile is configured for g++). Requires [libgdal](https://gdal.org/), [libosmium](https://osmcode.org/libosmium/), [boost](https://www.boost.org/) and [libconfig](https://github.com/hyperrealm/libconfig).

```
sudo apt install g++ libgdal-dev libosmium-dev libboost-all-dev libconfig-dev
git clone https://gitlab.gistools.geog.uni-heidelberg.de/giscience/openrouteservice-infrastructure/ors-preprocessor.git
cd ors-preprocessor
make
```

For the elevation data merge, also download the elevations tiles for CGIAR SRTM data (see [srtm.csi.cgiar.org](http://srtm.csi.cgiar.org/)) and GMTED2010 data (as a fallback option, see [GMTED2010](https://www.usgs.gov/land-resources/eros/coastal-changes-and-impacts/gmted2010)). Two python scripts are provided that download (and, in the case of CGIAR data, unpack) these resources. Be aware those require approx. 63.1 Gb and 26.3 Gb drive space respectively.

The downloaded files must be located in subdirs of where the tool is run from. In the desired directory, run
```
python getCGIAR.py
python getGMTED.py
```

## Usage
```
./ors-preprocessor [OPTIONS] [OSM file]

Options:
-m    do memory requirement check
-c    do only memory requirement check
-e    skip elevation data merge
-o    keep original elevation tags where present
```
The `ors-preprocessor.cfg` file is used to set up the tool. The default configuration is as follows:
```
# number of elevation tiles to keep open in memory simultaneously
cache_size = 10;

# regex for detecting tags that can safely be removed
remove_tag = "(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia";

# max IDs default values (fallback if memory check is skipped)
nodes_max_id = 11000000000L;
ways_max_id =   1200000000L;
rels_max_id =     20000000L;

#debug mode
#debug_output = true;
#debug_no_filter = true;
#debug_no_tag_filter = true;
```

- The `cache_size` determines how many tile GTIFs are kept open in memory during the processing. A too high number might result in paging overhead degrading performance.
- All tags matching the `remove_tag` regex are stripped from the data.
- The max IDs settings must be adjusted when OSM data grows. The configured number must be higher than the highest ID number for each category of elements in the source file. Note that memory consumption grows proportionally. The settings above are enough for current OSM planet file.
- The `debug_output` flag activates some verbose diagnostic output.
- The `debug_no_filter` flag deactivates filtering of elements, so that only metadata and tags are reduced.
- The `debug_no_tag_filter` flag deactivates filtering of tags, so that all tags are retained.

## Functionality details

The tool processes OSM data in two steps.

During the first pass, it filters way and relation elements and records the IDs and those of all referenced nodes where the elements are valid and used by ORS graphs. This process removes OSM ways representing objects unrelated to the road graph, with tags like "building", "landuse", "boundary", "natural" and a few others.

During the second pass, a new PBF file `[file].ors.pbf` is written, containing only the relevant elements for the ORS graphs. From the retained elements, all tags matching the `remove_tag` regular expression (see configuration file example above) as well as irrelevant metadata (version, user_id, timestamp etc.) are stripped.

Before the first pass, the tool needs to allocate the required memory to store the flags for the valid data sets in the source file. The max IDs numbers set in the config file need to be larger than the highest ID in the source file. You can use the option `-m` to let the tool determine the required memory automatically or option `-c` to only do the memory check to adjust the config values.

For each node, the preprocessor determines the elevation from the CGIAR data (if no data is available, falls back to GMTED data). You can skip this step by setting the `-e` option. Any `ele` tags already present in the OSM data are overwritten, since this is current ORS (and GH) behavior. You can pass the `-o` option to have the preprocessor retain the `ele` tag values where present in the OSM data. The elevation value defaults to 0 where it could not be determined (outside of tiles coverage or invalid data in both data sets), since this is GH behavior.

Overall, a ca. 70% file size reduction is achieved. Skipping the elevation data merge increases this only by a few percent, and since the elevation data retrieval during graph building (which can be skipped if the preprocessor already merged the elevation data) has a significant impact on memory consumption during graph building, the default behavior is to include this operation.

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
A log file is written containing all coordinates where no elevation value could be determined. The number of cases in the above example is normal (paths on Antarctica etc.).

## Limitations & future development
This tool is under development and still experimental, though it has been successfully tested on several OSM subsets (Germany, DACH). Use at own risk.
- Baden-Württemberg (from [geofabrik](http://download.geofabrik.de/europe/germany/baden-wuerttemberg.html)) runs in about 90 seconds on my ThinkPad, the OSM planet file in ca. 3 hours. The caching of elevation tile files might influence this, could be further optimized.
- The tags removal currently just removes a few typical ignorable tags, but a thorough statistical analysis of tags in OSM to identify the most frequently used tags could yield better results.  
- The elevation code is not precise in how the corresponding tiles file is determined from the queried coordinates, and needs further work. The current solution is in some cases effectively off by around 0.000139 degrees (ca. 10 m) and returns the value of a neighboring pixel, but since that error is smaller than the resolution of the elevation data itself and occurs only a small fraction of cases, it is arguable if this is a problem at all. Needs further investigation and potentially a fix.
- The downloading of elevation GTIF files might be integrated into the tool for convenience. It was just quicker to write a python script...

## Documentation
**Usage**:

```console
$ osm-transform [OPTIONS] COMMAND [ARGS]...
```

**Options**:

* `--logging [debug|info|warning|error|critical]`: [default: info]
* `--cores INTEGER`: Set the number of cores to use for processing.  [default: 14]
* `-v, --version`: Show the application's version and exit.
* `--install-completion`: Install completion for the current shell.
* `--show-completion`: Show completion for the current shell, to copy it or customize the installation.
* `--help`: Show this message and exit.

**Commands**:

* `docs`: Generate documentation
* `foo`

## `osm-transform docs`

Generate documentation

**Usage**:

```console
$ osm-transform docs [OPTIONS] COMMAND [ARGS]...
```

**Options**:

* `--help`: Show this message and exit.

**Commands**:

* `generate`: Generate markdown version of usage...

### `osm-transform docs generate`

Generate markdown version of usage documentation

**Usage**:

```console
$ osm-transform docs generate [OPTIONS]
```

**Options**:

* `--name TEXT`: The name of the CLI program to use in docs.
* `--output FILE`: An output file to write docs to, like README.md.
* `--help`: Show this message and exit.

## `osm-transform foo`

**Usage**:

```console
$ osm-transform foo [OPTIONS]
```

**Options**:

* `--help`: Show this message and exit.
