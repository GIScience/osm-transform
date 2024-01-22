cmake_minimum_required(VERSION 3.27)
project(osm-transform
        VERSION 0.1.0
        DESCRIPTION "Package to customize OSM files")

set(CMAKE_CXX_STANDARD 20)

# https://cmake.org/cmake/help/git-master/module/FindBoost.html
find_package(Boost REQUIRED system regex program_options)
find_package(GDAL CONFIG REQUIRED)

add_executable(osm-transform osm-transform.cpp node-locations.cpp ordered-index.cpp)

target_link_libraries(osm-transform PRIVATE z bz2 expat GDAL::GDAL Boost::system Boost::regex Boost::program_options)
