#include <boost/test/unit_test.hpp>

#include "location_elevation_service.h"

BOOST_AUTO_TEST_SUITE( test_locacion_elevation )
BOOST_AUTO_TEST_CASE( test_lookup ) {

    LocationElevationService location_elevation_service(1 << 20, false);
    location_elevation_service.load({"files/limburg_an_der_lahn.tif"});

    double ele = ((int)(location_elevation_service.elevation(osmium::Location(8.0513629, 50.3876977), false)*100))/100.0;
    BOOST_CHECK_EQUAL(ele, 163.81);

}

BOOST_AUTO_TEST_CASE( test_interpolate ) {

    LocationElevationService location_elevation_service(1 << 20, false);
    location_elevation_service.load({"files/limburg_an_der_lahn.tif"});

    auto interpolated = location_elevation_service.interpolate(osmium::Location(8.0515393,50.3873984), osmium::Location(8.0505023, 50.3868868));
    BOOST_CHECK_EQUAL(interpolated.size(), 14);
    for (const auto& le : interpolated) {
        auto location = le.location;
        auto ele = le.ele;
        std::cout << std::setprecision (8) << location.lon() << "," << location.lat() << "," << ele << "\n";
    }


}
BOOST_AUTO_TEST_SUITE_END()
