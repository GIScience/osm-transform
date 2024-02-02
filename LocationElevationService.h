#ifndef OSM_TRANSFORM_LOCATIONELEVATIONSERVICE_H
#define OSM_TRANSFORM_LOCATIONELEVATIONSERVICE_H

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

class GeoTiff;

struct prioAndFileName {
    double prio;
    std::string fileName;
};

class LocationElevationService {
    typedef boost::geometry::model::point<double, 2,  boost::geometry::cs::geographic< boost::geometry::degree>> point;
    typedef boost::geometry::model::box<point> box;
    typedef std::pair<box, prioAndFileName> rTreeEntry;

private:
    boost::geometry::index::rtree<rTreeEntry,  boost::geometry::index::quadratic<16>> rtree;
    std::unordered_map<std::string, std::shared_ptr<GeoTiff>> m_cache;
    std::list<std::string> m_lru;
    uint m_cache_size = 10;

    std::shared_ptr<GeoTiff> load_tiff(const char* filename);

public:
    explicit LocationElevationService();

    void load(const std::string &path);


    double elevation(osmium::Location l);
    std::vector<LocationElevation> interpolate(osmium::Location from, osmium::Location to);
};


#endif//OSM_TRANSFORM_LOCATIONELEVATIONSERVICE_H
