#include "test_utils.h"

void util_add_tags(osmium::builder::Builder &parent, std::map<std::string, std::string>& tags) {
    osmium::builder::TagListBuilder builder{parent};
    for (const auto& [key, value] : tags) {
        builder.add_tag(key, value);
    }
}
void util_add_refs(osmium::builder::Builder &parent, std::vector<osmium::object_id_type>& refs) {
    osmium::builder::WayNodeListBuilder builder{parent};
    for (const auto& ref : refs) {
        builder.add_node_ref(ref);
    }
}

void add_node(osmium::memory::Buffer &buffer, osmium::object_id_type id, std::map<std::string, std::string> tags, double lng, double lat) {
    osmium::builder::NodeBuilder builder{buffer};
    osmium::Node& obj = builder.object();
    obj.set_id(id);
    util_add_tags(builder, tags);
    obj.set_location(osmium::Location(lng, lat));
}

void add_way(osmium::memory::Buffer& buffer, osmium::object_id_type id, std::map<std::string, std::string> tags, std::vector<osmium::object_id_type> refs) {
    osmium::builder::WayBuilder builder(buffer);
    osmium::Way& obj = builder.object();
    obj.set_id(id);
    util_add_tags(builder, tags);
    util_add_refs(builder, refs);
}
