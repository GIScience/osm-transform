#ifndef OSM_TRANSFORM_TEST_UTILS_H
#define OSM_TRANSFORM_TEST_UTILS_H

#include <map>
#include <osmium/builder/osm_object_builder.hpp>
#include <osmium/memory/buffer.hpp>
#include <osmium/osm/node.hpp>

void util_add_tags(osmium::builder::Builder &parent, std::map<std::string, std::string>& tags);
void util_add_refs(osmium::builder::Builder &parent, std::vector<osmium::object_id_type>& refs);

void add_node(osmium::memory::Buffer &buffer, osmium::object_id_type id, std::map<std::string, std::string> tags, double lng, double lat);

void add_way(osmium::memory::Buffer& buffer, osmium::object_id_type id, std::map<std::string, std::string> tags, std::vector<osmium::object_id_type> refs);

#endif//OSM_TRANSFORM_TEST_UTILS_H
