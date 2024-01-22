#ifndef MAXIDHANDLER_H
#define MAXIDHANDLER_H
#include "FirstPassHandler.h"
#include "utils.h"


class MaxIDHandler : public osmium::handler::Handler {
public:
    llu node_max_id = 0;
    llu way_max_id = 0;
    llu relation_max_id = 0;

    void node(const osmium::Node &node) {
        if (node.id() < 0) return;
        node_max_id = node.id() > node_max_id ? node.id() : node_max_id;
    }

    void way(const osmium::Way &way) {
        if (way.id() < 0) return;
        way_max_id = way.id() > way_max_id ? way.id() : way_max_id;
        for (const auto &node_ref: way.nodes()) { node_max_id = node_ref.ref() > node_max_id ? node_ref.ref() : node_max_id; }
    }

    void relation(const osmium::Relation &rel) {
        if (rel.id() < 0) return;
        relation_max_id = rel.id() > relation_max_id ? rel.id() : relation_max_id;

        for (const auto &member: rel.members()) {
            if (member.type() == osmium::item_type::node) { node_max_id = member.ref() > node_max_id ? member.ref() : node_max_id; }
            if (member.type() == osmium::item_type::way) { way_max_id = member.ref() > way_max_id ? member.ref() : way_max_id; }
        }
    }
};


#endif //MAXIDHANDLER_H
