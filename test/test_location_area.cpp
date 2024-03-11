#include <boost/test/unit_test.hpp>

#include "../location_area_service.h"

BOOST_AUTO_TEST_SUITE( test_locacion_area )
BOOST_AUTO_TEST_CASE( test_location_area_service )
{
    std::string geo_type("wkt");
    std::string prefix("mapping_");
    LocationAreaService location_area_service(true, 0, 2, geo_type, true, prefix);
    location_area_service.load("test/mapping_test.csv");

    {
        const auto areas = location_area_service.get_area(osmium::Location(6.306152343750001, 50.05713877598692));
        BOOST_CHECK_EQUAL(areas.size(), 1);
        BOOST_CHECK_EQUAL(areas[0], "DEU");
    }

    {
        const auto areas = location_area_service.get_area(osmium::Location(6.0900938, 50.7225850));
        BOOST_CHECK_EQUAL(areas.size(), 1);
        BOOST_CHECK_EQUAL(areas[0], "DEU");
    }

    {
        const auto areas = location_area_service.get_area(osmium::Location(6.0902180,  50.7220057));
        BOOST_CHECK_EQUAL(areas.size(), 1);
        BOOST_CHECK_EQUAL(areas[0], "BEL");
    }


}
BOOST_AUTO_TEST_SUITE_END()