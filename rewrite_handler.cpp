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
            if (strcmp(key, "ele") == 0) {
                // keep ele tags only if no ele value passed
                if (ele == kNoDataValue) {
                    valid_tags_++;
                    std::string tagval(tag.value());
                    const auto value = regex_replace(tagval, non_digit_regex_, "");
                    builder.add_tag("ele", value);
                }
            } else {
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
        osmium::builder::NodeBuilder builder{*buffer_};
        builder.set_id(node.id());
        builder.set_location(node.location());
        double ele = kNoDataValue;
        if (add_elevation_) {
            if (!override_values_ && node.tags().has_key("ele")) {
                nodes_with_elevation_++;
            } else if ((ele = location_elevation_.elevation(node.location())) != kNoDataValue) {
                nodes_with_elevation_high_precision_++;
            } else if ((ele = getElevationCGIAR(node.location().lat(), node.location().lon())) != kNoDataValue) {
                nodes_with_elevation_srtm_precision_++;
            } else if ((ele = getElevationGMTED(node.location().lat(), node.location().lon())) != kNoDataValue) {
                nodes_with_elevation_gmted_precision_++;
            } else {
                nodes_with_elevation_not_found_++;
                ele = 0.0;// GH elevation code defaults to 0
            }
        }
        copy_tags(builder, node.tags(), ele);
        if (interpolate_) {
            location_index_->set(static_cast<osmium::unsigned_object_id_type>(node.id()), node.location());
        }
    }

    buffer_->commit();
}

void RewriteHandler::way(const osmium::Way &way) {
    if (way.id() < 0) return;
    if (valid_ids_.ways().get(way.id())) {
        processed_elements_++;
        osmium::builder::WayBuilder builder{*buffer_};
        builder.set_id(way.id());
        copy_tags(builder, way.tags());
        {
            osmium::builder::WayNodeListBuilder wnl_builder{builder};
            if (interpolate_) {
                interpolate(way, wnl_builder);
            } else {
                for (auto& ref : way.nodes()) {
                    wnl_builder.add_node_ref(ref);
                }
            }
        }
    }
    buffer_->commit();
}
void RewriteHandler::interpolate(const osmium::Way &way, osmium::builder::WayNodeListBuilder &wnl_builder) {
    auto from = way.nodes()[0];
    auto from_location = get_node_location(from.ref());
    wnl_builder.add_node_ref(from);
    for (int i = 1; i < way.nodes().size(); i++) {
        auto to = way.nodes()[i];
        if (interpolate_) {
            auto toLocation = get_node_location(to.ref());
            for (auto le: location_elevation_.interpolate(from_location, toLocation)) {
                auto new_node_id = next_node_id_++;
                newNode(new_node_id, le);
                wnl_builder.add_node_ref(new_node_id);
            }
            from_location = toLocation;
        }
        wnl_builder.add_node_ref(to);
        from = to;
    }
}

void RewriteHandler::newNode(osmium::object_id_type id, LocationElevation &le) {
    {
        osmium::builder::NodeBuilder nodeBuilder(*new_node_buffer_);
        nodeBuilder.set_id(id);
        nodeBuilder.set_location(le.location);
        {
            osmium::builder::TagListBuilder nodeTagsBuilder{nodeBuilder};
            nodeTagsBuilder.add_tag("ele", std::to_string(le.ele));
            nodeTagsBuilder.add_tag("highway", "traffic_signal");
        }
    }
    new_node_buffer_->commit();
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
double RewriteHandler::getElevationCGIAR(const double lat, const double lng, const bool debug) {
    const int lng_index = floor(1 + (180 + lng) / 5);
    const int lat_index = floor(1 + (60 - lat) / 5);
    char filename[100];
    snprintf(filename, 24, "srtmdata/srtm_%02d_%02d.tif", lng_index, lat_index);
    if (debug) printf("Filename for coordinates %.6f - %.6f : %s\n", lng, lat, filename);
    return getElevationFromFile(lat, lng, filename);
}
double RewriteHandler::getElevationGMTED(const double lat, const double lng, const bool debug) {
    const int lng_index = static_cast<int>(-180 + floor((180 + lng) / 30) * 30);
    const int lat_index = static_cast<int>(-70 + floor((70 + lat) / 20) * 20);
    const char lng_pre = lng_index < 0 ? 'W' : 'E';
    const char lat_pre = lat_index < 0 ? 'S' : 'N';
    char filename[100];
    snprintf(filename, 44, "gmteddata/%02d%c%03d%c_20101117_gmted_mea075.tif", abs(lat_index), lat_pre, abs(lng_index),
             lng_pre);
    if (debug) printf("Filename for coordinates %.6f - %.6f : %s\n", lng, lat, filename);
    return getElevationFromFile(lat, lng, filename);
}

double RewriteHandler::getElevationFromFile(const double lat, const double lng, char *filename) {
    const auto geo_tiff = location_elevation_.load_tiff(filename);
    if (geo_tiff == nullptr)
        return kNoDataValue;
    return geo_tiff->elevation(lng, lat);
}
