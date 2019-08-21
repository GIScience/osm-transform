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
./ors-preprocessor [OSM file] [-s]
```
The tool processes OSM data in two steps. During the first pass, it filters way and relation elements and records their IDs and those of all referenced nodes.

The `-s` flag (currently not implemented as actual flag check but rather by third argv present) only does the first processing step and  outputs the detected numbers of valid elements of each type. The count of nodes here is only an estimate, since in this mode the calculations do not take into account the fact that nodes might be referenced by multiple ways/relations.

These numbers can be used to configure memory preallocation to speed up further processing. The `ors-preprocessor.cfg` file is used to set up the tool.

During the second pass, a new PBF file `[file].ors.pbf` is written, containing only the relevant elements for the ORS graphs. This also removes all metadata (timestamps, user info, version etc.) from all elements and strips away unneeded tags (configurable, tags like notes, URLs, source info etc.).

## Limitations
This tool is under development and still experimental. Use at own risk.
- On smaller files this tool already works well, Baden-Württemberg (from [geofabrik](http://download.geofabrik.de/europe/germany/baden-wuerttemberg.html)) runs in about 40 seconds on my ThinkPad.
- For OSM planet file, the `valid_nodes` set has to contain close to 3 billion node ID entries (long long = 8 bytes each, so something like 22.3 Gb). Need to see if a better solution can be found than putting them all in an `unordered_set`.
- Need to detect lack of memory, dying with a `bad_alloc` is not good...

## TODO
- test with reasonably big graphs if routing results stay the same
- test & profile on server with enough memory with PLANET
- make tags for validating/invalidating ways&relations configurable
- ...