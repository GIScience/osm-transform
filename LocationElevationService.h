#ifndef OSM_TRANSFORM_LOCATIONELEVATIONSERVICE_H
#define OSM_TRANSFORM_LOCATIONELEVATIONSERVICE_H


#include <boost/geometry/geometries/box.hpp>
#include <boost/geometry/geometries/point.hpp>
#include <boost/geometry/index/rtree.hpp>
#include <osmium/osm/location.hpp>


struct LocationElevation {
    osmium::Location location;
    double ele;
};

struct prioAndFileName;

class LocationElevationService {
    typedef boost::geometry::model::point<double, 2,  boost::geometry::cs::geographic< boost::geometry::degree>> point;
    typedef boost::geometry::model::box<point> box;
    typedef std::pair<box, prioAndFileName> rTreeEntry;

private:
    boost::geometry::index::rtree<rTreeEntry,  boost::geometry::index::quadratic<16>> rtree;

public:

    void load(const std::string &path);


    std::vector<LocationElevation> interpolate(osmium::Location from, osmium::Location to);
};


#endif//OSM_TRANSFORM_LOCATIONELEVATIONSERVICE_H
