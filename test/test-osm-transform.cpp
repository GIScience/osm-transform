#define BOOST_TEST_MODULE FirstPassHandler Test
#include <boost/test/included/unit_test.hpp>
#include <boost/regex.hpp>

#include <osmium/osm/tag.hpp>
#include <osmium/memory/buffer.hpp>
#include <osmium/builder/osm_object_builder.hpp>

#define private public
#include "../firstpass_handler.h"
#include "../location_area_service.h"


BOOST_AUTO_TEST_CASE(test_has_no_relevant_tags)
{
  boost::regex remove_tags(
    "(.*:)?remove_1(:.*)?|remove_2",
    boost::regex::icase
  );
  osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> valid_ids;
  osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> no_elevation;
  FirstPassHandler fph(remove_tags, valid_ids, no_elevation);

  // Create tag list with relevant and irrelevant tags
  osmium::memory::Buffer buf1{1024, osmium::memory::Buffer::auto_grow::yes};
  osmium::builder::TagListBuilder tlb(buf1);
  tlb.add_tag("landuse","forest");
  tlb.add_tag("railway", "platform");
  osmium::TagList &tags_with_relevant_tags = buf1.get<osmium::TagList>(0);
  BOOST_TEST(fph.has_no_relevant_tags(tags_with_relevant_tags) == false);

  // Create tag list with only irrelevant tags
  osmium::memory::Buffer buf2{1024, osmium::memory::Buffer::auto_grow::yes};
  osmium::builder::TagListBuilder tlb2(buf2);
  tlb2.add_tag("landuse","forest");
  osmium::TagList &tags_without_relevant_tags = buf2.get<osmium::TagList>(0);
  BOOST_TEST(fph.has_no_relevant_tags(tags_without_relevant_tags) == true);
}

BOOST_AUTO_TEST_CASE(test_location_area_service)
{
    std::string geo_type("wkt");
    std::string prefix("mapping_");
    LocationAreaService location_area_service(true, 0, 2, geo_type, true, prefix);
    location_area_service.load("test/mapping_test.csv");

    osmium::Location location = osmium::Location(6.306152343750001, 50.05713877598692);
    std::vector<std::string> areas = location_area_service.get_area(location);

    BOOST_TEST(areas.size() == 1);
    BOOST_TEST(areas[0] == "DEU");
}
