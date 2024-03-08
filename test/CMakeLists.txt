cmake_minimum_required(VERSION 3.22)

find_package(Boost REQUIRED COMPONENTS unit_test_framework regex)
find_package(GDAL REQUIRED)

file(GLOB sources CONFIGURE_DEPENDS *.h *.cpp)

add_executable(test-osm-transform test-osm-transform.cpp ../location_area_service.cpp)
target_link_libraries(test-osm-transform PRIVATE Boost::unit_test_framework Boost::regex GDAL::GDAL)

enable_testing()


