cmake_minimum_required(VERSION 3.16)
project(osm-transform)


set(CMAKE_CXX_STANDARD 14)


if (CMAKE_HOST_WIN32)
    set(libname "libconfig")
else ()
    set(libname "config")
endif ()

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=c++11 -m64")
add_executable(osm-transform osm-transform.cpp)
target_link_libraries(osm-transform ${libname}++ pthread z expat bz2 config++ gdal boost_regex boost_system)
include_directories(${CPLUS_INCLUDE_PATH} ${C_INCLUDE_PATH} /usr/include/gdal)

set(EXECUTABLE_OUTPUT_PATH ${PROJECT_SOURCE_DIR}/)
