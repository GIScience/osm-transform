#ifndef REWRITEHANDLER_H
#define REWRITEHANDLER_H
#include "GeoTiff.h"
#include "utils.h"
#include <osmium/handler.hpp>
#include <boost/regex.hpp>

#include <iostream>

class RewriteHandler : public osmium::handler::Handler {
    friend ostream &operator<<(ostream &out, const RewriteHandler &handler);

    osmium::memory::Buffer *m_buffer;
    vi *valid_nodes;
    vi *valid_ways;
    vi *valid_relations;
    boost::regex *remove_tags;
    boost::regex non_digit_regex;
    bool DEBUG_NO_FILTER = false;
    bool DEBUG_NO_TAG_FILTER = false;


    unordered_map<string, GeoTiff *> elevationData;
    int cache_size = -1;
    list<string> cache_queue;
    ofstream *log;

    osmium::memory::Buffer *m_new_node_buffer;
    osmid_t m_next_node_id;
    std::unique_ptr<osmium::index::map::Map<osmium::unsigned_object_id_type, osmium::Location>> &m_location_index;

    void copy_tags(osmium::builder::Builder &parent, const osmium::TagList &tags, const int ele = NO_DATA_VALUE) {
        osmium::builder::TagListBuilder builder{parent};
        for (const auto &tag: tags) {
            total_tags++;
            if (DEBUG_NO_TAG_FILTER || !boost::regex_match(tag.key(), *remove_tags)) {
                string key = tag.key();
                if (key == "ele") {
                    // keep ele tags only if no ele value passed
                    if (ele == NO_DATA_VALUE) {
                        valid_tags++;
                        string tagval(tag.value());
                        string value = regex_replace(tagval, non_digit_regex, "");
                        builder.add_tag("ele", value);
                    }
                } else {
                    valid_tags++;
                    builder.add_tag(tag);
                }
            }
        }
        if (ele > NO_DATA_VALUE) { builder.add_tag("ele", to_string(ele)); }
    }

    double getElevationCGIAR(const double lat, const double lng, const bool debug = false) {
        const int lngIndex = floor(1 + (180 + lng) / 5);
        const int latIndex = floor(1 + (60 - lat) / 5);
        char pszFilename[100];
        snprintf(pszFilename, 24, "srtmdata/srtm_%02d_%02d.tif", lngIndex, latIndex);
        if (debug) printf("Filename for coordinates %.6f - %.6f : %s\n", lng, lat, pszFilename);
        return getElevationFromFile(lat, lng, pszFilename, debug);
    }

    double getElevationGMTED(const double lat, const double lng, const bool debug = false) {
        const int lngIndex = static_cast<int>(-180 + floor((180 + lng) / 30) * 30);
        const int latIndex = static_cast<int>(-70 + floor((70 + lat) / 20) * 20);
        const char lngPre = lngIndex < 0 ? 'W' : 'E';
        const char latPre = latIndex < 0 ? 'S' : 'N';
        char pszFilename[100];
        snprintf(pszFilename, 44, "gmteddata/%02d%c%03d%c_20101117_gmted_mea075.tif", abs(latIndex), latPre, abs(lngIndex),
                 lngPre);
        if (debug) printf("Filename for coordinates %.6f - %.6f : %s\n", lng, lat, pszFilename);
        return getElevationFromFile(lat, lng, pszFilename, debug);
    }

    GeoTiff* get_geotiff_from_cache(char *pszFilename, const bool debug) {

        const auto search = elevationData.find(pszFilename);
        if (search != elevationData.end()) {
            const auto geoTiff = static_cast<GeoTiff*>(elevationData.at(pszFilename));
            cache_queue.remove(pszFilename);
            cache_queue.emplace_front(pszFilename);
            return geoTiff;
        }

        if (!file_exists(pszFilename)) {
            return nullptr;
            // if (debug) cout << "File does not exist: " << pszFilename << endl;
            // value = NO_DATA_VALUE;
            // return true;
        }
        auto geoTiff = new GeoTiff(pszFilename);
        if (geoTiff == nullptr) {
            return nullptr;
            // if (debug) cout << "Failed to read input data from file " << pszFilename << endl;
            // value = NO_DATA_VALUE;
            // return true;
        }
        elevationData.insert(make_pair(pszFilename, geoTiff));
        if (cache_queue.size() == cache_size) {
            elevationData.erase(cache_queue.back());
            cache_queue.pop_back();
        }
        if (debug)
            printf("Dataset opened. (format: %s; size: %d x %d x %d)\n", geoTiff->GetDescription(),
                   geoTiff->GetRasterXSize(), geoTiff->GetRasterYSize(), geoTiff->GetRasterCount());
        cache_queue.emplace_front(pszFilename);

        return geoTiff;
    }

    double getElevationFromFile(const double lat, const double lng, char *pszFilename, const bool debug = false) {
        const auto geo_tiff = get_geotiff_from_cache(pszFilename, debug);
        if (geo_tiff == nullptr)
            return NO_DATA_VALUE;
        return geo_tiff->elevation(lng, lat);
    }

    auto get_node_location(const osmium::object_id_type id) -> osmium::Location {
        return m_location_index->get_noexcept(static_cast<osmium::unsigned_object_id_type>(id));
    }

public:
    llu valid_elements = 0;
    llu processed_elements = 0;
    llu total_tags = 0;
    llu valid_tags = 0;
    bool addElevation = false;
    bool overrideValues = false;
    llu nodes_with_elevation_srtm_precision = 0;
    llu nodes_with_elevation_gmted_precision = 0;
    llu nodes_with_elevation = 0;
    llu nodes_with_elevation_not_found = 0;

    explicit RewriteHandler(const osmid_t next_node_id, std::unique_ptr<osmium::index::map::Map<osmium::unsigned_object_id_type, osmium::Location>> &location_index)
        : m_next_node_id(next_node_id), m_location_index(location_index) {}

    void set_buffer(osmium::memory::Buffer *buffer) {
        m_buffer = buffer;
        valid_elements = 0;
        processed_elements = 0;
        total_tags = 0;
        valid_tags = 0;
    }

    void set_new_node_buffer(osmium::memory::Buffer *buffer) { m_new_node_buffer = buffer; }

    void init(const int i_cache_size, boost::regex *re, vi *i_valid_nodes, vi *i_valid_ways, vi *i_valid_relations,
              ofstream *logref, const bool debug_no_filter, const bool debug_no_tag_filter) {
        cache_size = i_cache_size;
        remove_tags = re;
        valid_nodes = i_valid_nodes;
        valid_ways = i_valid_ways;
        valid_relations = i_valid_relations;
        valid_elements = valid_nodes->size() + valid_ways->size() + valid_relations->size();
        log = logref;
        DEBUG_NO_FILTER = debug_no_filter;
        DEBUG_NO_TAG_FILTER = debug_no_tag_filter;
        non_digit_regex = boost::regex("[^0-9.]");
    }

    void node(const osmium::Node &node) {
        processed_elements++;
        if (node.id() < 0) return;
        {
            if (DEBUG_NO_FILTER || testBit(*valid_nodes, node.id()) > 0) {
                osmium::builder::NodeBuilder builder{*m_buffer};
                builder.set_id(node.id());
                builder.set_location(node.location());
                int ele = NO_DATA_VALUE;
                if (addElevation) {
                    if (!overrideValues && node.tags().has_key("ele")) { nodes_with_elevation++; } else {
                        ele = getElevationCGIAR(node.location().lat(), node.location().lon());
                        if (ele != NO_DATA_VALUE) { nodes_with_elevation_srtm_precision++; } else {
                            ele = getElevationGMTED(node.location().lat(), node.location().lon());
                            if (ele != NO_DATA_VALUE) { nodes_with_elevation_gmted_precision++; } else {
                                nodes_with_elevation_not_found++;
                                *log << getTimeStr() << " ele retrieval failed: " << node.location().lat() << " "
                                        << node.location().lon() << endl;
                                ele = 0.0;// GH elevation code defaults to 0
                            }
                        }
                    }
                }
                copy_tags(builder, node.tags(), ele);
                m_location_index->set(static_cast<osmium::unsigned_object_id_type>(node.id()), node.location());
            }
        }
        m_buffer->commit();
    }

    void way(const osmium::Way &way) {
        processed_elements++;
        if (way.id() < 0) return;
        if (DEBUG_NO_FILTER || testBit(*valid_ways, way.id()) > 0) {
            auto next_node_id = m_next_node_id;

            {
                osmium::builder::WayBuilder builder{*m_buffer};
                builder.set_id(way.id());
                copy_tags(builder, way.tags());
                {
                    osmium::builder::WayNodeListBuilder wnl_builder{builder};
                    auto from = way.nodes()[0];
                    auto fromLocation = get_node_location(from.positive_ref());
                    wnl_builder.add_node_ref(from);
                    for (int i = 1; i < way.nodes().size(); i++) {
                        auto to = way.nodes()[i];
                        auto toLocation = get_node_location(to.positive_ref());
                        wnl_builder.add_node_ref(to);

                        // from / to with locations
                        //  split(fromLocation, toLocation);

                        from = to;
                        fromLocation = toLocation;
                    }
                }
            }
//            for (const auto &node: nodes) {
//                {
//                    osmium::builder::NodeBuilder nodeBuilder(*m_new_node_buffer);
//                    nodeBuilder.set_id(next_node_id++);
//                    nodeBuilder.set_location(osmium::Location(node.x(), node.y()));
//                    {
//                        osmium::builder::TagListBuilder nodeTagsBuilder{nodeBuilder};
//                        nodeTagsBuilder.add_tag("ele", to_string(node.elevation()));
//                    }
//                }
//                m_new_node_buffer->commit();
//            }
        }
        m_buffer->commit();
    }

    void relation(const osmium::Relation &relation) {
        processed_elements++;
        if (relation.id() < 0) return;
        {
            if (DEBUG_NO_FILTER || testBit(*valid_relations, relation.id()) > 0) {
                osmium::builder::RelationBuilder builder{*m_buffer};
                builder.set_id(relation.id());
                builder.add_item(relation.members());
                copy_tags(builder, relation.tags());
            }
        }
        m_buffer->commit();
    }
};


#endif //REWRITEHANDLER_H
