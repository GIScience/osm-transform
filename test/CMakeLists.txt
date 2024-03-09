cmake_minimum_required(VERSION 3.22)

find_package(Boost REQUIRED COMPONENTS unit_test_framework regex)
find_package(GDAL REQUIRED)

add_executable(test-osm-transform test-osm-transform.cpp
        ../location_area_service.cpp
        ../location_elevation_service.cpp
        ../rewrite_handler.cpp
        test_utils.cpp
        test_location_area.cpp
        test_location_elevation.cpp
        test_firstpass_handler.cpp
        test_rewrite_handler.cpp
)
target_link_libraries(test-osm-transform PRIVATE Boost::unit_test_framework Boost::regex GDAL::GDAL)

enable_testing()


