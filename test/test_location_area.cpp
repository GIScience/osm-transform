#include <boost/test/unit_test.hpp>

#include "../location_area_service.h"

BOOST_AUTO_TEST_CASE(test_location_area_service)
{
    std::string geo_type("wkt");
    std::string prefix("mapping_");
    LocationAreaService location_area_service(true, 0, 2, geo_type, true, prefix);
    location_area_service.load("test/mapping_test.csv");

    osmium::Location location = osmium::Location(6.306152343750001, 50.05713877598692);
    std::vector<std::string> areas = location_area_service.get_area(location);

    BOOST_TEST(areas.size() == 1);
    BOOST_TEST(areas[0] == "DEU");
}