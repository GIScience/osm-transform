#ifndef OSM_TRANSFORM_LOCATION_ELEVATION_SERVICE_H
#define OSM_TRANSFORM_LOCATION_ELEVATION_SERVICE_H

#include <list>

#include <boost/geometry.hpp>
#include <boost/geometry/geometries/box.hpp>
#include <boost/geometry/geometries/point.hpp>
#include <boost/geometry/index/rtree.hpp>

#include <osmium/osm/location.hpp>


struct LocationElevation {
    osmium::Location location;
    double ele;
};

class Geotiff;

struct PrioAndFilename {
    double prio;
    std::string filename;
};

class LocationElevationService {
    typedef boost::geometry::model::point<double, 2,  boost::geometry::cs::geographic< boost::geometry::degree>> point;
    typedef boost::geometry::model::box<point> box;
    typedef std::pair<box, PrioAndFilename> rtree_entry;

private:
    boost::geometry::index::rtree<rtree_entry,  boost::geometry::index::quadratic<16>> rtree_;
    std::unordered_map<std::string, std::shared_ptr<Geotiff>> cache_;
    std::list<std::string> lru_;
    uint cache_size_ = 10;

public:

    explicit LocationElevationService(uint cache_size);

    void load(const std::string &path);

    std::shared_ptr<Geotiff> load_tiff(const char* filename);

    double elevation(osmium::Location l);

    std::vector<LocationElevation> interpolate(osmium::Location from, osmium::Location to);
};


#endif//OSM_TRANSFORM_LOCATION_ELEVATION_SERVICE_H