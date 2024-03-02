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
    ulong mem_size_ = 0;
    ulong cache_limit_ = 150000000;
    std::map<std::string, std::uint64_t> tile_size_;
    bool debug_mode_ = false;

public:
    unsigned long long found_custom_ = 0;
    unsigned long long found_srtm_ = 0;
    unsigned long long found_gmted_ = 0;

    explicit LocationElevationService(ulong cache_limit, bool debug_mode);

    void load(const std::vector<std::string> &paths);

    std::shared_ptr<Geotiff> load_tiff(const char* filename);

    double elevation(osmium::Location l, bool count);

    std::vector<LocationElevation> interpolate(osmium::Location from, osmium::Location to);

};


#endif//OSM_TRANSFORM_LOCATION_ELEVATION_SERVICE_H
