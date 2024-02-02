#include "RewriteHandler.h"

#include <osmium/builder/osm_object_builder.hpp>
#include <osmium/osm/node.hpp>
#include <osmium/osm/relation.hpp>
#include <osmium/osm/tag.hpp>
#include <osmium/osm/way.hpp>

void RewriteHandler::copy_tags(osmium::builder::Builder &parent, const osmium::TagList &tags, const double ele) {
    osmium::builder::TagListBuilder builder{parent};
    for (const auto &tag: tags) {
        total_tags++;
        if (!boost::regex_match(tag.key(), remove_tags)) {
            const auto key = tag.key();
            if (strcmp(key, "ele") == 0) {
                // keep ele tags only if no ele value passed
                if (ele == NO_DATA_VALUE) {
                    valid_tags++;
                    std::string tagval(tag.value());
                    const auto value = regex_replace(tagval, non_digit_regex, "");
                    builder.add_tag("ele", value);
                }
            } else {
                valid_tags++;
                builder.add_tag(tag);
            }
        }
    }
    if (ele > NO_DATA_VALUE) { builder.add_tag("ele", std::to_string(ele)); }
}

void RewriteHandler::node(const osmium::Node &node) {
    if (node.id() < 0) return;
    if (m_valid_ids.nodes().get(node.id())) {
        processed_elements++;
        osmium::builder::NodeBuilder builder{*m_buffer};
        builder.set_id(node.id());
        builder.set_location(node.location());
        double ele = NO_DATA_VALUE;
        if (addElevation) {
            if (!overrideValues && node.tags().has_key("ele")) {
                nodes_with_elevation++;
            } else if ((ele = location_elevation.elevation(node.location())) != NO_DATA_VALUE) {
                nodes_with_elevation_high_precision++;
            } else if ((ele = getElevationCGIAR(node.location().lat(), node.location().lon())) != NO_DATA_VALUE) {
                nodes_with_elevation_srtm_precision++;
            } else if ((ele = getElevationGMTED(node.location().lat(), node.location().lon())) != NO_DATA_VALUE) {
                nodes_with_elevation_gmted_precision++;
            } else {
                nodes_with_elevation_not_found++;
                ele = 0.0;// GH elevation code defaults to 0
            }
        }
        copy_tags(builder, node.tags(), ele);
        m_location_index->set(static_cast<osmium::unsigned_object_id_type>(node.id()), node.location());
    }

    m_buffer->commit();
}

void RewriteHandler::way(const osmium::Way &way) {
    if (way.id() < 0) return;
    if (m_valid_ids.ways().get(way.id())) {
        processed_elements++;
        osmium::builder::WayBuilder builder{*m_buffer};
        builder.set_id(way.id());
        copy_tags(builder, way.tags());
        {
            osmium::builder::WayNodeListBuilder wnl_builder{builder};
            auto from = way.nodes()[0];
            auto fromLocation = get_node_location(from.ref());
            wnl_builder.add_node_ref(from);
            for (int i = 1; i < way.nodes().size(); i++) {
                auto to = way.nodes()[i];
                auto toLocation = get_node_location(to.ref());

                for (auto le: location_elevation.interpolate(fromLocation, toLocation)) {

                    auto new_node_id = m_next_node_id++;
                    newNode(new_node_id, le);
                    m_new_node_buffer->commit();
                    wnl_builder.add_node_ref(new_node_id);
                }


                wnl_builder.add_node_ref(to);
                from = to;
                fromLocation = toLocation;
            }
        }
    }
    m_buffer->commit();
}

void RewriteHandler::newNode(osmium::object_id_type id, LocationElevation &le) {
    osmium::builder::NodeBuilder nodeBuilder(*m_new_node_buffer);
    nodeBuilder.set_id(id);
    nodeBuilder.set_location(le.location);
    {
        osmium::builder::TagListBuilder nodeTagsBuilder{nodeBuilder};
        nodeTagsBuilder.add_tag("ele", std::to_string(le.ele));
        nodeTagsBuilder.add_tag("highway", "traffic_signal");
    }
}

void RewriteHandler::relation(const osmium::Relation &relation) {
    if (relation.id() < 0) return;
    if (m_valid_ids.relations().get(relation.id())) {
        processed_elements++;
        osmium::builder::RelationBuilder builder{*m_buffer};
        builder.set_id(relation.id());
        builder.add_item(relation.members());
        copy_tags(builder, relation.tags());
    }
    m_buffer->commit();
}
double RewriteHandler::getElevationCGIAR(const double lat, const double lng, const bool debug) {
    const int lngIndex = floor(1 + (180 + lng) / 5);
    const int latIndex = floor(1 + (60 - lat) / 5);
    char pszFilename[100];
    snprintf(pszFilename, 24, "srtmdata/srtm_%02d_%02d.tif", lngIndex, latIndex);
    if (debug) printf("Filename for coordinates %.6f - %.6f : %s\n", lng, lat, pszFilename);
    return getElevationFromFile(lat, lng, pszFilename);
}
double RewriteHandler::getElevationGMTED(const double lat, const double lng, const bool debug) {
    const int lngIndex = static_cast<int>(-180 + floor((180 + lng) / 30) * 30);
    const int latIndex = static_cast<int>(-70 + floor((70 + lat) / 20) * 20);
    const char lngPre = lngIndex < 0 ? 'W' : 'E';
    const char latPre = latIndex < 0 ? 'S' : 'N';
    char pszFilename[100];
    snprintf(pszFilename, 44, "gmteddata/%02d%c%03d%c_20101117_gmted_mea075.tif", abs(latIndex), latPre, abs(lngIndex),
             lngPre);
    if (debug) printf("Filename for coordinates %.6f - %.6f : %s\n", lng, lat, pszFilename);
    return getElevationFromFile(lat, lng, pszFilename);
}
double RewriteHandler::getElevationFromFile(const double lat, const double lng, char *pszFilename) {
    const auto geo_tiff = location_elevation.load_tiff(pszFilename);
    if (geo_tiff == nullptr)
        return NO_DATA_VALUE;
    return geo_tiff->elevation(lng, lat);
}
