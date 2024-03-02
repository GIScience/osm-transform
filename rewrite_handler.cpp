#include "rewrite_handler.h"

#include <osmium/builder/osm_object_builder.hpp>
#include <osmium/osm/node.hpp>
#include <osmium/osm/relation.hpp>
#include <osmium/osm/tag.hpp>
#include <osmium/osm/way.hpp>

void RewriteHandler::copy_tags(osmium::builder::Builder &parent, const osmium::TagList &tags, const double ele) {
    osmium::builder::TagListBuilder builder{parent};
    for (const auto &tag: tags) {
        total_tags_++;
        if (!boost::regex_match(tag.key(), remove_tags_)) {
            const auto key = tag.key();
            if (strcmp(key, "ele") != 0 || !add_elevation_) {
                valid_tags_++;
                builder.add_tag(tag);
            }
        }
    }
    if (ele > kNoDataValue) { builder.add_tag("ele", std::to_string(ele)); }
}

void RewriteHandler::node(const osmium::Node &node) {
    if (node.id() < 0) return;
    if (valid_ids_.nodes().get(node.id())) {
        processed_elements_++;
        osmium::builder::NodeBuilder builder{*node_buffer_};
        builder.set_id(node.id());
        builder.set_location(node.location());
        double ele = kNoDataValue;
        if (add_elevation_ ) { //&& !no_elevation_.nodes().get(node.id())) {
            if ((ele = location_elevation_.elevation(node.location(), true)) != kNoDataValue) {
                nodes_with_elevation_++;
            } else {
                nodes_with_elevation_not_found_++;
            }
        }
        copy_tags(builder, node.tags(), ele);
        if (interpolate_) {
            location_index_->set(static_cast<osmium::unsigned_object_id_type>(node.id()), node.location());
        }
    }

    node_buffer_->commit();
}

void RewriteHandler::way(const osmium::Way &way) {
    if (way.id() < 0) return;
    if (valid_ids_.ways().get(way.id())) {
        processed_elements_++;
        osmium::builder::WayBuilder builder{*buffer_};
        builder.set_id(way.id());
        copy_tags(builder, way.tags());
        add_refs(way, builder);
    }
    buffer_->commit();
}
void RewriteHandler::add_refs(const osmium::Way &way, osmium::builder::Builder &builder) {
    osmium::builder::WayNodeListBuilder wnl_builder{builder};
    if (interpolate_ && !no_elevation_.ways().get(way.id())) {
        interpolate(way, wnl_builder);
        return;
    }
    for (auto& ref : way.nodes()) {
        wnl_builder.add_node_ref(ref);
    }
}

void RewriteHandler::interpolate(const osmium::Way &way, osmium::builder::WayNodeListBuilder &wnl_builder) {
    auto from = way.nodes()[0];
    auto from_location = get_node_location(from.ref());
    wnl_builder.add_node_ref(from);
    for (int i = 1; i < way.nodes().size(); i++) {
        auto to = way.nodes()[i];
        auto to_location = get_node_location(to.ref());
        auto les = location_elevation_.interpolate(from_location, to_location);
        for (int index = 1; index < les.size() -1; ++index) {
            auto before_ele = les.at(index - 1).ele;
            auto after_ele = les.at(index + 1).ele;
            auto le = les.at(index);
            if (le.ele == kNoDataValue)  {
                continue;
            }
            if (abs(le.ele - (before_ele + after_ele) / 2) >= interpolate_threshold_) {
                auto new_node_id = next_node_id_++;
                newNode(new_node_id, le);
                wnl_builder.add_node_ref(new_node_id);
            }
        }
        from_location = to_location;
        wnl_builder.add_node_ref(to);
        from = to;
    }
}

void RewriteHandler::newNode(osmium::object_id_type id, LocationElevation &le) {
    {
        osmium::builder::NodeBuilder nodeBuilder(*node_buffer_);
        nodeBuilder.set_id(id);
        nodeBuilder.set_location(le.location);
        {
            osmium::builder::TagListBuilder nodeTagsBuilder{nodeBuilder};
            nodeTagsBuilder.add_tag("ele", std::to_string(le.ele));
        }
    }
    nodes_added_by_interpolation_++;
    node_buffer_->commit();
}

void RewriteHandler::relation(const osmium::Relation &relation) {
    if (relation.id() < 0) return;
    if (valid_ids_.relations().get(relation.id())) {
        processed_elements_++;
        osmium::builder::RelationBuilder builder{*buffer_};
        builder.set_id(relation.id());
        builder.add_item(relation.members());
        copy_tags(builder, relation.tags());
    }
    buffer_->commit();
}
