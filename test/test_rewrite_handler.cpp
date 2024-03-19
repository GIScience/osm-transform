#include <boost/test/unit_test.hpp>

#include <osmium/memory/buffer.hpp>
#include <osmium/visitor.hpp>

#include "rewrite_handler.h"

#include "test_utils.h"

BOOST_AUTO_TEST_SUITE( test_rewrite_pass )
BOOST_AUTO_TEST_CASE (interpolation_0_5) {
    auto debug_mode = false;
    std::string geo_type{"wkt"};
    std::string prefix{"mapping_"};
    std::string index_type{"flex_mem"};

    const auto &map_factory = osmium::index::MapFactory<osmium::unsigned_object_id_type, osmium::Location>::instance();
    auto location_index = map_factory.create_map(index_type);

    LocationElevationService location_elevation_service(1 << 20, debug_mode);
    location_elevation_service.load({"files/limburg_an_der_lahn.tif"});

    LocationAreaService location_area_service(debug_mode, 0, 2, geo_type, true, prefix);
    location_area_service.load("test/mapping_test.csv");

    osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> valid_ids;
    osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> no_elevation;
    boost::regex remove_tag_regex("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia", boost::regex::icase);
    auto interpolate = true;
    auto interpolate_threshold = 0.5;

    osmium::memory::Buffer input{1 << 10, osmium::memory::Buffer::auto_grow::yes};

    add_node(input, 101, {}, 8.0515393, 50.3873984);
    add_node(input, 102, {}, 8.0505023, 50.3868868);
    valid_ids.nodes().set(101);
    valid_ids.nodes().set(102);
    add_way(input, 10, {}, {101, 102});
    valid_ids.ways().set(10);

    input.commit();

    RewriteHandler handler(1000000000, location_index, location_elevation_service, location_area_service, remove_tag_regex, valid_ids, no_elevation, interpolate, interpolate_threshold);
    osmium::memory::Buffer output_nodes{1 << 10, osmium::memory::Buffer::auto_grow::yes};
    osmium::memory::Buffer output_ways{1 << 10, osmium::memory::Buffer::auto_grow::yes};
    handler.set_buffers(&output_ways, &output_nodes);
    osmium::apply(input, handler);

    {
        auto nodes = output_nodes.select<osmium::Node>();
        BOOST_CHECK_EQUAL(nodes.size(), 11);
        //        auto item = nodes.begin();
        //        {
        //            const auto& node = (*item++);
        //            BOOST_CHECK_EQUAL(node.id(), 101);
        //            BOOST_CHECK(node.tags().empty());
        //        }
    }
    {
        auto ways = output_ways.select<osmium::Way>();
        BOOST_CHECK_EQUAL(ways.size(), 1);
    }
}

BOOST_AUTO_TEST_CASE (interpolation_1_0) {
    auto debug_mode = false;
    std::string geo_type{"wkt"};
    std::string prefix{"mapping_"};
    std::string index_type{"flex_mem"};

    const auto &map_factory = osmium::index::MapFactory<osmium::unsigned_object_id_type, osmium::Location>::instance();
    auto location_index = map_factory.create_map(index_type);

    LocationElevationService location_elevation_service(1 << 20, debug_mode);
    location_elevation_service.load({"files/limburg_an_der_lahn.tif"});

    LocationAreaService location_area_service(debug_mode, 0, 2, geo_type, true, prefix);
    location_area_service.load("test/mapping_test.csv");

    osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> valid_ids;
    osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> no_elevation;
    boost::regex remove_tag_regex("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia", boost::regex::icase);
    auto interpolate = true;
    auto interpolate_threshold = 1.0;

    osmium::memory::Buffer input{1 << 10, osmium::memory::Buffer::auto_grow::yes};

    add_node(input, 101, {}, 8.0515393, 50.3873984);
    add_node(input, 102, {}, 8.0505023, 50.3868868);
    valid_ids.nodes().set(101);
    valid_ids.nodes().set(102);
    add_way(input, 10, {}, {101, 102});
    valid_ids.ways().set(10);

    input.commit();

    RewriteHandler handler(1000000000, location_index, location_elevation_service, location_area_service, remove_tag_regex, valid_ids, no_elevation, interpolate, interpolate_threshold);
    osmium::memory::Buffer output_nodes{1 << 10, osmium::memory::Buffer::auto_grow::yes};
    osmium::memory::Buffer output_ways{1 << 10, osmium::memory::Buffer::auto_grow::yes};
    handler.set_buffers(&output_ways, &output_nodes);
    osmium::apply(input, handler);

    {
        auto nodes = output_nodes.select<osmium::Node>();
        BOOST_CHECK_EQUAL(nodes.size(), 6);
    }
    {
        auto ways = output_ways.select<osmium::Way>();
        BOOST_CHECK_EQUAL(ways.size(), 1);
    }
}

BOOST_AUTO_TEST_CASE (interpolation_10_0) {
    auto debug_mode = false;
    std::string geo_type{"wkt"};
    std::string prefix{"mapping_"};
    std::string index_type{"flex_mem"};

    const auto &map_factory = osmium::index::MapFactory<osmium::unsigned_object_id_type, osmium::Location>::instance();
    auto location_index = map_factory.create_map(index_type);

    LocationElevationService location_elevation_service(1 << 20, debug_mode);
    location_elevation_service.load({"files/limburg_an_der_lahn.tif"});

    LocationAreaService location_area_service(debug_mode, 0, 2, geo_type, true, prefix);
    location_area_service.load("test/mapping_test.csv");

    osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> valid_ids;
    osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> no_elevation;
    boost::regex remove_tag_regex("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia", boost::regex::icase);
    auto interpolate = true;
    auto interpolate_threshold = 10.0;

    osmium::memory::Buffer input{1 << 10, osmium::memory::Buffer::auto_grow::yes};

    add_node(input, 101, {}, 8.0515393, 50.3873984);
    add_node(input, 102, {}, 8.0505023, 50.3868868);
    valid_ids.nodes().set(101);
    valid_ids.nodes().set(102);
    add_way(input, 10, {}, {101, 102});
    valid_ids.ways().set(10);

    input.commit();

    RewriteHandler handler(1000000000, location_index, location_elevation_service, location_area_service, remove_tag_regex, valid_ids, no_elevation, interpolate, interpolate_threshold);
    osmium::memory::Buffer output_nodes{1 << 10, osmium::memory::Buffer::auto_grow::yes};
    osmium::memory::Buffer output_ways{1 << 10, osmium::memory::Buffer::auto_grow::yes};
    handler.set_buffers(&output_ways, &output_nodes);
    osmium::apply(input, handler);

    {
        auto nodes = output_nodes.select<osmium::Node>();
        BOOST_CHECK_EQUAL(nodes.size(), 2);
    }
    {
        auto ways = output_ways.select<osmium::Way>();
        BOOST_CHECK_EQUAL(ways.size(), 1);
    }
}

BOOST_AUTO_TEST_CASE (rewrite_full_pass) {
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
    osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> no_elevation;
    boost::regex remove_tag_regex("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia", boost::regex::icase);
    auto interpolate = false;
    auto interpolate_threshold = 0.5;
    RewriteHandler handler(1000000000, location_index, location_elevation_service, location_area_service, remove_tag_regex, valid_ids, no_elevation, interpolate, interpolate_threshold);

    osmium::memory::Buffer input{1 << 10, osmium::memory::Buffer::auto_grow::yes};

    add_node(input, 101, {{"fixme","name"}, {"FIXME","yes"}, {"FixME", "check"}}, 0.0, 0.0);
    valid_ids.nodes().set(101);
    no_elevation.nodes().set(101);
    add_node(input, 102, {{"ors:source", "transform"}, {"note:check", "yes"}}, 0.0, 0.0);
    valid_ids.nodes().set(102);
    no_elevation.nodes().set(102);

    add_node(input, 201, {}, 0.0, 0.0);
    add_node(input, 202, {}, 0.0, 0.0);

    add_node(input, 301, {{"highway","crossing"}}, 0.0, 0.0);
    valid_ids.nodes().set(301);
    add_node(input, 302, {}, 0.0, 0.0);
    valid_ids.nodes().set(302);
    add_node(input, 91142609, {}, 6.0902180, 50.7220057);
    valid_ids.nodes().set(91142609);
    add_node(input, 270418052, {}, 8.6761206, 49.4181246);
    valid_ids.nodes().set(270418052);
    add_node(input, 278110816, {}, 6.0900938, 50.7225850);
    valid_ids.nodes().set(278110816);
    add_node(input, 6772131501, {},8.6760972, 49.4181246);
    valid_ids.nodes().set(6772131501);

    add_way(input, 10, {{"highway","yes"}}, { 101, 102 });
    valid_ids.ways().set(10);
    add_way(input, 30, {{"highway","yes"}}, { 301, 302 });
    valid_ids.ways().set(30);

    add_way(input,721933838, {{"highway", "primary"}, {"name", "Berliner Straße"}}, { 270418052, 721933838 });
    valid_ids.ways().set(721933838);
    input.commit();

    osmium::memory::Buffer output_nodes{1 << 10, osmium::memory::Buffer::auto_grow::yes};
    osmium::memory::Buffer output_ways{1 << 10, osmium::memory::Buffer::auto_grow::yes};
    handler.set_buffers(&output_ways, &output_nodes);
    osmium::apply(input, handler);

    {
        auto nodes = output_nodes.select<osmium::Node>();
        BOOST_CHECK_EQUAL(nodes.size(), 8);
        auto item = nodes.begin();
        {
            const auto& node = (*item++);
            BOOST_CHECK_EQUAL(node.id(), 101);
            BOOST_CHECK(node.tags().empty());
        }
        {
            const auto &node = (*item++);
            BOOST_CHECK_EQUAL(node.id(), 102);
            BOOST_CHECK(node.tags().empty());
        }
        {
            const auto &node = (*item++);
            BOOST_CHECK_EQUAL(node.id(), 301);
            BOOST_CHECK_EQUAL(node.tags().size(), 1);
            BOOST_CHECK(node.tags().has_tag("highway", "crossing"));
        }
        {
            const auto &node = (*item++);
            BOOST_CHECK_EQUAL(node.id(), 302);
        }
        {
            const auto &node = (*item++);
            BOOST_CHECK_EQUAL(node.id(), 91142609);
            BOOST_CHECK_EQUAL(node.tags().size(), 1);
            BOOST_CHECK_EQUAL(node.tags().get_value_by_key("country",""), "BEL");
        }
        {
            const auto &node = (*item++);
            BOOST_CHECK_EQUAL(node.id(), 270418052);
            BOOST_CHECK_EQUAL(node.tags().size(), 1);
            BOOST_CHECK_EQUAL(node.tags().get_value_by_key("country",""), "DEU");
        }
        {
            const auto &node = (*item++);
            BOOST_CHECK_EQUAL(node.id(), 278110816);
            BOOST_CHECK_EQUAL(node.tags().size(), 1);
            BOOST_CHECK_EQUAL(node.tags().get_value_by_key("country",""), "DEU");
        }
        {
            const auto &node = (*item++);
            BOOST_CHECK_EQUAL(node.id(), 6772131501);
            BOOST_CHECK_EQUAL(node.tags().size(), 1);
            BOOST_CHECK_EQUAL(node.tags().get_value_by_key("country",""), "DEU");
        }
    }

    {
        auto ways = output_ways.select<osmium::Way>();
        BOOST_CHECK_EQUAL(ways.size(), 3);
        auto item = ways.begin();
        {
            const auto& way = (*item++);
            BOOST_CHECK_EQUAL(way.id(), 10);
            BOOST_CHECK_EQUAL(way.tags().size(), 1);
        }
        {
            const auto& way = (*item++);
            BOOST_CHECK_EQUAL(way.id(), 30);
        }
        {
            const auto& way = (*item++);
            BOOST_CHECK_EQUAL(way.id(), 721933838);
            BOOST_CHECK_EQUAL(way.tags().size(), 2);
        }
    }

}

BOOST_AUTO_TEST_SUITE_END()
