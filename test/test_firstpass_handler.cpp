#include <boost/test/unit_test.hpp>

#include <boost/regex.hpp>

#include <osmium/osm/tag.hpp>
#include <osmium/builder/osm_object_builder.hpp>
#include <osmium/memory/buffer.hpp>
#include <osmium/visitor.hpp>

#define private public
#include "../firstpass_handler.h"
#include "test_utils.h"

BOOST_AUTO_TEST_SUITE( test_first_pass )

BOOST_AUTO_TEST_CASE(test_has_no_relevant_tags) {
    boost::regex remove_tags(
            "(.*:)?remove_1(:.*)?|remove_2",
            boost::regex::icase);
    osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> valid_ids;
    osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> no_elevation;
    FirstPassHandler handler(remove_tags, valid_ids, no_elevation);

    // Create tag list with relevant and irrelevant tags
    osmium::memory::Buffer buf1{1024, osmium::memory::Buffer::auto_grow::yes};
    osmium::builder::TagListBuilder tlb(buf1);
    tlb.add_tag("landuse", "forest");
    tlb.add_tag("railway", "platform");
    auto &tags_with_relevant_tags = buf1.get<osmium::TagList>(0);
    BOOST_TEST(handler.has_no_relevant_tags(tags_with_relevant_tags) == false);

    // Create tag list with only irrelevant tags
    osmium::memory::Buffer buf2{1024, osmium::memory::Buffer::auto_grow::yes};
    osmium::builder::TagListBuilder tlb2(buf2);
    tlb2.add_tag("landuse", "forest");
    auto &tags_without_relevant_tags = buf2.get<osmium::TagList>(0);
    BOOST_TEST(handler.has_no_relevant_tags(tags_without_relevant_tags) == true);
}

BOOST_AUTO_TEST_CASE(bla) {
    boost::regex remove_tags("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia", boost::regex::icase);
    osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> valid_ids;
    osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> no_elevation;
    FirstPassHandler handler(remove_tags, valid_ids, no_elevation);

    osmium::memory::Buffer buffer{1024, osmium::memory::Buffer::auto_grow::yes};
    //    add_node(buffer, 123, {{"troilo", "rafael"}, {"highway", "yes"}}, 8.6756824, 49.4184793);
    //    add_node(buffer, 234, {{"troilo", "rafael"}, {"building", "bla"}}, 8.6756824, 49.4184793);
    add_way(buffer, 12, {{"highway","yes"}}, {123, 234});
    buffer.commit();

    osmium::apply(buffer, handler);
    BOOST_TEST(valid_ids.nodes().size() == 2);
    BOOST_TEST(valid_ids.ways().size() == 1);
    BOOST_TEST(valid_ids.nodes().get(123));
    BOOST_TEST(valid_ids.nodes().get(234));
    BOOST_TEST(valid_ids.ways().get(12));

}

BOOST_AUTO_TEST_SUITE_END()