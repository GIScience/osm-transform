#ifndef FIRSTPASSHANDLER_H
#define FIRSTPASSHANDLER_H

#include <ostream>
#include <set>
#include <boost/regex.hpp>
#include <osmium/handler.hpp>
#include <osmium/index/id_set.hpp>
#include <osmium/index/nwr_array.hpp>
#include "utils.h"

using namespace std;

class FirstPassHandler : public osmium::handler::Handler {
    friend std::ostream &operator<<(std::ostream &out, const FirstPassHandler &handler);

    const std::set<std::string> invalidating_tags{"building", "landuse", "boundary", "natural", "place", "waterway", "aeroway",
                                                  "aviation", "military", "power", "communication", "man_made"};
    // const set<string> invalidating_tags{"building", "landuse"};
    boost::regex &remove_tags;

    static bool validating_tags(const string &tag, const string &value) {
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
            if (!boost::regex_match(tag.key(), remove_tags)) {
                tag_count++;
                if (validating_tags(tag.key(), tag.value())) { return false; } else if (invalidating_tags.contains(tag.key())) { is_removable = true; }
            }
        }
        return tag_count == 0 || is_removable;
    }

public:
    llu node_count = 0;
    llu relation_count = 0;
    llu way_count = 0;

    osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &m_valid_ids;

    explicit FirstPassHandler(boost::regex &re, osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids) :
            remove_tags(re),
            m_valid_ids(valid_ids) {}

    void node(const osmium::Node &node) {
        if (node.id() < 0) return;
        node_count++;
    }

    void way(const osmium::Way &way) {
        if (way.id() < 0) return;
        way_count++;
        if (way.nodes().size() < 2 || check_tags(way.tags())) { return; }
        for (const osmium::NodeRef &n: way.nodes()) {
            m_valid_ids.nodes().set(n.ref());
        }
        m_valid_ids.ways().set(way.id());
    }

    void relation(const osmium::Relation &rel) {
        if (rel.id() < 0) return;
        relation_count++;
        if (check_tags(rel.tags())) { return; }
        for (const auto &member: rel.members()) {
            if (member.type() == osmium::item_type::node) {
                m_valid_ids.nodes().set(member.ref());
            }
        }
        m_valid_ids.relations().set(rel.id());
    }
};


#endif //FIRSTPASSHANDLER_H
