add_executable(${PROJECT_NAME} ${SOURCE_FILES})

target_link_libraries(${PROJECT_NAME} PRIVATE z bz2 expat GDAL::GDAL Boost::regex Boost::program_options)

install(TARGETS osm-transform DESTINATION bin)