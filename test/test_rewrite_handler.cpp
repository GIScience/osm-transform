#include <boost/test/unit_test.hpp>

#include <osmium/memory/buffer.hpp>
#include <osmium/visitor.hpp>

#include "../rewrite_handler.h"

#include "test_utils.h"

BOOST_AUTO_TEST_SUITE( test_rewrite_pass )

BOOST_AUTO_TEST_CASE (pass) {
    auto debug_mode = false;
    std::string geo_type{"wkt"};
    std::string prefix{"mapping_"};
    std::string index_type{"flex_mem"};

    const auto& map_factory = osmium::index::MapFactory<osmium::unsigned_object_id_type, osmium::Location>::instance();
    auto location_index = map_factory.create_map(index_type);

    LocationElevationService location_elevation_service(1 << 20, debug_mode);
    LocationAreaService location_area_service(debug_mode, 0, 2, geo_type, true, prefix);
    location_area_service.load("test/mapping_test.csv");

    osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> valid_ids;
    valid_ids.nodes().set(101);
    valid_ids.nodes().set(102);
    valid_ids.nodes().set(301);
    valid_ids.nodes().set(302);
    valid_ids.ways().set(10);
    valid_ids.ways().set(30);

    osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> no_elevation;
    no_elevation.nodes().set(101);
    no_elevation.nodes().set(102);

    boost::regex remove_tag_regex("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia", boost::regex::icase);

    auto interpolate = false;
    auto interpolate_threshold = 0.5;
    RewriteHandler handler(1000000000, location_index, location_elevation_service, location_area_service, remove_tag_regex, valid_ids, no_elevation, interpolate, interpolate_threshold);

    osmium::memory::Buffer input{1 << 10, osmium::memory::Buffer::auto_grow::yes};
    add_node(input, 101, {}, 0.0, 0.0);
    add_node(input, 102, {}, 0.0, 0.0);
    add_node(input, 201, {}, 0.0, 0.0);
    add_node(input, 202, {}, 0.0, 0.0);
    add_node(input, 301, {}, 0.0, 0.0);
    add_node(input, 302, {}, 0.0, 0.0);
    input.commit();


    osmium::memory::Buffer output_nodes{1 << 10, osmium::memory::Buffer::auto_grow::yes};
    osmium::memory::Buffer output_ways{1 << 10, osmium::memory::Buffer::auto_grow::yes};
    handler.set_buffers(&output_ways, &output_nodes);
    osmium::apply(input, handler);

    BOOST_CHECK_EQUAL(4, output_nodes.select<osmium::Node>().size());

    auto iter = output_nodes.select<osmium::Node>();
    auto item = iter.begin();
    BOOST_CHECK_EQUAL(101, (*item++).id());
    // check elevation, check country, check tags
    BOOST_CHECK_EQUAL(102, (*item++).id());
    BOOST_CHECK_EQUAL(301, (*item++).id());
    BOOST_CHECK_EQUAL(302, (*item++).id());

}

BOOST_AUTO_TEST_SUITE_END()