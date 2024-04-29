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
