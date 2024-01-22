#ifndef FIRSTPASSHANDLER_H
#define FIRSTPASSHANDLER_H
#include "NodeWithElevation.h"

#include <ostream>
#include <set>
#include <boost/regex.hpp>
#include <osmium/handler.hpp>
#include "utils.h"

using namespace std;

class FirstPassHandler : public osmium::handler::Handler {
    friend std::ostream &operator<<(std::ostream &out, const FirstPassHandler &ce);

    const std::set<std::string> invalidating_tags{"building", "landuse", "boundary", "natural", "place", "waterway", "aeroway",
                                                  "aviation", "military", "power", "communication", "man_made"};
    // const set<string> invalidating_tags{"building", "landuse"};
    boost::regex *remove_tags;
    llu node_max_id = 0;
    llu way_max_id = 0;
    llu relation_max_id = 0;

    static void exitSegfault(const string &type, const llu id) {
        printf("%s ID %lld exceeds the allocated flag memory. Please increase the value in the config file. \nTo determine the exact value required, run this tool with the -c option.\n",
               type.c_str(), id);
        exit(4);
    }

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
            if (!boost::regex_match(tag.key(), *remove_tags)) {
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

    vi *valid_nodes;
    vi *valid_ways;
    vi *valid_relations;

    bool DEBUG_NO_FILTER = false;

    void init(boost::regex *re, vi *i_valid_nodes, vi *i_valid_ways, vi *i_valid_relations, const bool debug_no_filter,
              const llu i_node_max_id, const llu i_way_max_id, const llu i_relation_max_id) {
        remove_tags = re;
        valid_nodes = i_valid_nodes;
        valid_ways = i_valid_ways;
        valid_relations = i_valid_relations;
        DEBUG_NO_FILTER = debug_no_filter;
        node_max_id = i_node_max_id;
        way_max_id = i_way_max_id;
        relation_max_id = i_relation_max_id;
    }

    void node(const osmium::Node &node) {
        if (node.id() < 0) return;
        if (node.id() > node_max_id) { exitSegfault("Node", node.id()); }
        node_count++;
    }

    void way(const osmium::Way &way) {
        if (way.id() < 0) return;
        if (way.id() > way_max_id) { exitSegfault("Way", way.id()); }
        way_count++;
        if (DEBUG_NO_FILTER || way.id() < 0 || way.nodes().size() < 2 || check_tags(way.tags())) { return; }
        for (const osmium::NodeRef &n: way.nodes()) { setBit(*valid_nodes, n.ref()); }
        setBit(*valid_ways, way.id());
    }

    void relation(const osmium::Relation &rel) {
        if (rel.id() < 0) return;
        if (rel.id() > relation_max_id) { exitSegfault("Relation", rel.id()); }
        relation_count++;
        if (DEBUG_NO_FILTER || rel.id() < 0 || check_tags(rel.tags())) { return; }
        for (const auto &member: rel.members()) { if (member.type() == osmium::item_type::node) { setBit(*valid_nodes, member.ref()); } }
        setBit(*valid_relations, rel.id());
    }
};


#endif //FIRSTPASSHANDLER_H
