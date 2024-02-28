cmake_minimum_required(VERSION 3.22)
project(osm-transform
        VERSION 0.1.0
        DESCRIPTION "Package to customize OSM files")
# expose name & version
add_definitions( -DPROJECT_VERSION="${PROJECT_VERSION}" -DPROJECT_NAME="${PROJECT_NAME}")

# ignore BOOST deprecated headers
add_definitions("-DBOOST_ALLOW_DEPRECATED_HEADERS")

set(CMAKE_CXX_STANDARD 23)

# https://cmake.org/cmake/help/git-master/module/FindBoost.html
find_package(Boost REQUIRED system regex program_options)
find_package(GDAL REQUIRED)

file(GLOB sources CONFIGURE_DEPENDS *.h *.cpp)
add_executable(${PROJECT_NAME} ${sources})

target_link_libraries(${PROJECT_NAME} PRIVATE z bz2 expat GDAL::GDAL Boost::system Boost::regex Boost::program_options)

# unit tests
add_subdirectory(test)
