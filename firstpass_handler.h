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
    const std::set<std::string> kInvalidatingTags{"building", "landuse",
        "boundary", "natural", "place", "waterway", "aeroway",
        "aviation", "military", "power", "communication", "man_made"};
    boost::regex &remove_tags_;

    unsigned long long node_count_ = 0;
    unsigned long long relation_count_ = 0;
    unsigned long long way_count_ = 0;

    static bool tag_validates(const osmium::Tag &tag) {
        const std::string key = tag.key();
        const std::string value = tag.value();

        if (key == "highway") return true;
        if (key == "route") return true;
        if (key == "railway" && value == "platform") return true;
        if (key == "public_transport" && value == "platform") return true;
        if (key == "man_made" && value == "pier") return true;
        return false;
    }

    inline bool accept_tag(const osmium::Tag &tag) const {
        return !boost::regex_match(tag.key(), remove_tags_);
    }

    bool has_no_relevant_tags(const osmium::TagList &tags) const {
        bool no_tags_remain = true;
        bool has_invalidating_tags = false;
        for (const osmium::Tag &tag: tags) {
            if (accept_tag(tag)) {
                no_tags_remain = false;
                if (tag_validates(tag)) {
                    return false;
                } else if (kInvalidatingTags.contains(tag.key())) {
                    has_invalidating_tags = true;
                }
            }
        }
        return no_tags_remain || has_invalidating_tags;
    }

    inline bool is_removable(const osmium::Way &way) const {
        return way.nodes().size() < 2 || has_no_relevant_tags(way.tags());
    }

    inline bool is_removable(const osmium::Relation &rel) const {
        return has_no_relevant_tags(rel.tags());
    }

public:
    osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids_;

    explicit FirstPassHandler(
        boost::regex &remove_tags,
        osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids
    ): remove_tags_(remove_tags),
       valid_ids_(valid_ids)
    {}

    void node(const osmium::Node &node) {
        if (node.id() < 0) return;
        node_count_++;
    }

    void way(const osmium::Way &way) {
        if (way.id() < 0) return;
        way_count_++;
        if (is_removable(way)) { return; }
        for (const osmium::NodeRef &n: way.nodes()) {
            valid_ids_.nodes().set(n.ref());
        }
        valid_ids_.ways().set(way.id());
    }

    void relation(const osmium::Relation &rel) {
        if (rel.id() < 0) return;
        relation_count_++;
        if (is_removable(rel)) { return; }
        for (const auto &member: rel.members()) {
            if (member.type() == osmium::item_type::node) {
                valid_ids_.nodes().set(member.ref());
            }
        }
        valid_ids_.relations().set(rel.id());
    }
    void printStats() {
        std::cout << "valid nodes: " << valid_ids_.nodes().size() << " (" << node_count_ << "), "
            << "valid ways: " << valid_ids_.ways().size() << " (" << way_count_ << "), "
            << "valid relations: " << valid_ids_.relations().size() << " (" << relation_count_ << ")"
            << std::endl;
    };
};

#endif //FIRSTPASSHANDLER_H
