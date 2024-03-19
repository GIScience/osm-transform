find_package(Boost REQUIRED COMPONENTS unit_test_framework regex)
find_package(GDAL REQUIRED)

include_directories(../src)

set(TEST_FILES 
  test_utils.cpp
  test_location_area.cpp
  test_location_elevation.cpp
  test_firstpass_handler.cpp
  test_rewrite_handler.cpp
)

foreach(file ${SOURCE_FILES})
  list(APPEND TEST_FILES "../src/${file}")
endforeach()

add_executable(test-osm-transform test-osm-transform.cpp
        ${TEST_FILES}
)
target_link_libraries(test-osm-transform PRIVATE Boost::unit_test_framework Boost::regex GDAL::GDAL)
