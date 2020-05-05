# ors-preprocessor
Tool for reduction of OSM data and processing cost for elevation data extraction during graph building. Removes metadata, unused elements and tags, and adds `ele` tags to all retained nodes.

## Installation
Clone and `make` (Makefile is configured for g++). Requires [libgdal](https://gdal.org/), [libosmium](https://osmcode.org/libosmium/) and [boost](https://www.boost.org/).

On Ubuntu:
```
sudo apt install g++ libgdal-dev libosmium-dev libboost-all-dev
make
```

For the elevation data merge, also download the elevations tiles for CGIAR SRTM data (see [srtm.csi.cgiar.org](http://srtm.csi.cgiar.org/)) and GMTED2010 data (as a fallback option, see [GMTED2010](https://www.usgs.gov/land-resources/eros/coastal-changes-and-impacts/gmted2010)). Two python scripts are provided that download (and, in the case of CGIAR data, unpack) these resources. Be aware those require approx. 63.1 Gb and 26.3 Gb drive space respectively.

The downloaded files must be located in subdirs of where the tool is run from.
```
python getCGIAR.py
python getGMTED.py
```

## Installation caveat: GDAL
On some systems, installing [GDAL](https://gdal.org/download.html) manually is required, which in turn requires compiling current version of [PROJ.4](https://proj.org/download.html) (Ubuntu 19.10). Download both packages, unpack, then in the PROJ dir
```
./configure
make
sudo make install
```
then in the GDAL dir
```
CPPFLAGS=-I/usr/local/include LDFLAGS=-L/usr/local/lib ./configure --with-proj=/usr/local
make
sudo make install
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
nodes_max_id = 7300000000L;
ways_max_id =   800000000L;
rels_max_id =    20000000L;

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
- The elevation code is not precise in how the corresponding tiles file is determined from the queried coordinates, and needs further work. The current solution is in some cases effectively off by around 0.000137 degrees (ca. 10 m) and returns the value of a neighboring pixel, but since that error is smaller than the resolution of the elevation data itself and occurs only a small fraction of cases, it is arguable if this is a problem at all. Needs further investigation and potentially a fix.
- The downloading of elevation GTIF files might be integrated into the tool for convenience. It was just quicker to write a python script...
