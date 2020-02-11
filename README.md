# ors-preprocessor
Tool for OSM data reduction - remove metadata, unused elements and tags.

## Installation
Clone and `make` (Makefile is configured for g++). Requires [libosmium](https://osmcode.org/libosmium/) and [boost](https://www.boost.org/).
On Ubuntu:
```
sudo apt install g++ libosmium-dev libboost-all-dev
make
```

## Usage
```
./ors-preprocessor [OSM file]
```
The tool processes OSM data in two steps. During the first pass, it filters way and relation elements and records their IDs and those of all referenced nodes.

The `ors-preprocessor.cfg` file is used to set up the tool. The default configuration is as follows:
```
# regex for detecting tags that can safely be removed
remove_tag = "(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia";

# max IDs
nodes_max_id = 6800000000L;
ways_max_id =   800000000L;
rels_max_id =    10000000L;

#debug mode
#debug_output = true;
#debug_no_filter = true;
#debug_no_tag_filter = true;
```

During the second pass, a new PBF file `[file].ors.pbf` is written, containing only the relevant elements for the ORS graphs. From the retained elements, all tags matching the `remove_tag` regular expression (see configuration file example above) as well as irrelevant metadata (version, user_id, timestamp etc.) are stripped.

Overall, a 60-65% file size reduction is achieved.

## Limitations
This tool is under development and still experimental. Use at own risk.
- Baden-Württemberg (from [geofabrik](http://download.geofabrik.de/europe/germany/baden-wuerttemberg.html)) runs in about 35 seconds on my ThinkPad, the OSM planet file in under an hour.
- The max IDs settings must be adjusted when OSM data grows. The configured number must be higher than the highest ID number for each category of elements. Note that memory consumption grows proportionally. The settings above are enough for current OSM planet file.

## TODO
- test with reasonably big graphs if routing results stay the same
- make tags for way/relation filtering configurable

## WIP
Installing [GDAL](https://gdal.org/download.html) requires compiling current version of [PROJ.4](https://proj.org/download.html) (Ubuntu 19.10). Download both packages, unpack, then in the PROJ dir
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
