cmake_minimum_required(VERSION 3.22)
project(osm-transform
        VERSION 0.1.0
        DESCRIPTION "Package to customize OSM files")
# expose name & version
add_definitions( -DPROJECT_VERSION="${PROJECT_VERSION}" -DPROJECT_NAME="${PROJECT_NAME}")

# ignore BOOST deprecated headers
add_definitions("-DBOOST_ALLOW_DEPRECATED_HEADERS")

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CONFIGURATION_TYPES "Debug,Release" CACHE STRING "Configuration Types" FORCE)

option(BUILD_TESTING "Build the tests" ON)

# https://cmake.org/cmake/help/git-master/module/FindBoost.html
find_package(Boost REQUIRED regex program_options)
find_package(GDAL REQUIRED)
find_package(ZLIB REQUIRED)
find_package(CURL REQUIRED)

include_directories(${PROJECT_BINARY_DIR}/src)

set(SOURCE_FILES
    location_area_service.cpp
    location_elevation_service.cpp
    rewrite_handler.cpp
)

add_subdirectory(src)
target_link_libraries(${PROJECT_NAME} PRIVATE z minizip bz2 expat GDAL::GDAL Boost::regex Boost::program_options CURL::libcurl)

if (BUILD_TESTING)
    enable_testing()
    add_subdirectory(test)
endif()


