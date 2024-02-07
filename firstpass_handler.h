#ifndef FIRSTPASSHANDLER_H
#define FIRSTPASSHANDLER_H

#include <ostream>
#include <set>

#include <boost/regex.hpp>

#include <osmium/handler.hpp>
#include <osmium/index/id_set.hpp>
#include <osmium/index/nwr_array.hpp>
#include <osmium/osm/tag.hpp>
#include <osmium/osm/node.hpp>
#include <osmium/osm/types.hpp>
#include <osmium/osm/way.hpp>
#include <osmium/osm/relation.hpp>

class FirstPassHandler : public osmium::handler::Handler {
    friend std::ostream &operator<<(std::ostream &out, const FirstPassHandler &handler);

    const std::set<std::string> kInvalidatingTags{"building", "landuse", "boundary", "natural", "place", "waterway", "aeroway",
                                                  "aviation", "military", "power", "communication", "man_made"};
    // const set<string> kInvalidatingTags{"building", "landuse"};
    boost::regex &remove_tags_;

    static bool validating_tags(const std::string &tag, const std::string &value) {
        if (tag == "highway") return true;
        if (tag == "route") return true;
        if (tag == "railway" && value == "platform") return true;
        if (tag == "public_transport" && value == "platform") return true;
        if (tag == "man_made" && value == "pier") return true;
        return false;
    }

    bool check_tags(const osmium::TagList &tags) const {
        int tag_count = 0;
        bool is_removable = false;
        for (const osmium::Tag &tag: tags) {
            if (!boost::regex_match(tag.key(), remove_tags_)) {
                tag_count++;
                if (validating_tags(tag.key(), tag.value())) { return false; } else if (kInvalidatingTags.contains(tag.key())) { is_removable = true; }
            }
        }
        return tag_count == 0 || is_removable;
    }

public:
    unsigned long long node_count_ = 0;
    unsigned long long relation_count_ = 0;
    unsigned long long way_count_ = 0;

    osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids_;

    explicit FirstPassHandler(boost::regex &remove_tags, osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids) : remove_tags_(remove_tags),
                                                                                                                                            valid_ids_(valid_ids) {}

    void node(const osmium::Node &node) {
        if (node.id() < 0) return;
        node_count_++;
    }

    void way(const osmium::Way &way) {
        if (way.id() < 0) return;
        way_count_++;
        if (way.nodes().size() < 2 || check_tags(way.tags())) { return; }
        for (const osmium::NodeRef &n: way.nodes()) {
            valid_ids_.nodes().set(n.ref());
        }
        valid_ids_.ways().set(way.id());
    }

    void relation(const osmium::Relation &rel) {
        if (rel.id() < 0) return;
        relation_count_++;
        if (check_tags(rel.tags())) { return; }
        for (const auto &member: rel.members()) {
            if (member.type() == osmium::item_type::node) {
                valid_ids_.nodes().set(member.ref());
            }
        }
        valid_ids_.relations().set(rel.id());
    }
};


#endif //FIRSTPASSHANDLER_H
