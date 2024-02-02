#ifndef REWRITEHANDLER_H
#define REWRITEHANDLER_H

#include <filesystem>
#include <iostream>

#include <boost/regex.hpp>

#include <osmium/handler.hpp>
#include <osmium/index/id_set.hpp>
#include <osmium/index/map/all.hpp>
#include <osmium/index/node_locations_map.hpp>
#include <osmium/index/nwr_array.hpp>
#include <osmium/memory/buffer.hpp>

#include "LocationElevationService.h"
#include "GeoTiff.h"

class RewriteHandler : public osmium::handler::Handler {

    osmium::memory::Buffer *m_buffer;
    osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &m_valid_ids;

    boost::regex &remove_tags;
    const boost::regex non_digit_regex = boost::regex("[^0-9.]");

    osmium::memory::Buffer *m_new_node_buffer;
    osmium::object_id_type m_next_node_id;
    std::unique_ptr<osmium::index::map::Map<osmium::unsigned_object_id_type, osmium::Location>> &m_location_index;
    LocationElevationService &location_elevation;

    void copy_tags(osmium::builder::Builder &parent, const osmium::TagList &tags, const double ele = NO_DATA_VALUE);

    double getElevationCGIAR(const double lat, const double lng, const bool debug = false);

    double getElevationGMTED(const double lat, const double lng, const bool debug = false);

    double getElevationFromFile(const double lat, const double lng, char *pszFilename);

    auto get_node_location(const osmium::object_id_type id) -> osmium::Location {
        return m_location_index->get_noexcept(static_cast<osmium::unsigned_object_id_type>(id));
    }

    void newNode(osmium::object_id_type id, LocationElevation &le);

public:
    unsigned long long processed_elements = 0;
    unsigned long long total_tags = 0;
    unsigned long long valid_tags = 0;
    bool addElevation = false;
    bool overrideValues = false;
    unsigned long long nodes_with_elevation_srtm_precision = 0;
    unsigned long long nodes_with_elevation_high_precision = 0;
    unsigned long long nodes_with_elevation_gmted_precision = 0;
    unsigned long long nodes_with_elevation = 0;
    unsigned long long nodes_with_elevation_not_found = 0;

    explicit RewriteHandler(const osmium::object_id_type next_node_id, std::unique_ptr<osmium::index::map::Map<osmium::unsigned_object_id_type, osmium::Location>> &location_index,
                            LocationElevationService &elevation_service,
                            boost::regex &re, osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids) :
            m_next_node_id(next_node_id),
            m_location_index(location_index),
            location_elevation(elevation_service),
            remove_tags(re),
            m_valid_ids(valid_ids) {
    }

    void set_buffers(osmium::memory::Buffer *output_buffer, osmium::memory::Buffer *output_node_buffer) {
        m_buffer = output_buffer;
        m_new_node_buffer = output_node_buffer;
        processed_elements = 0;
        total_tags = 0;
        valid_tags = 0;
    }

    void node(const osmium::Node &node);

    void way(const osmium::Way &way);

    void relation(const osmium::Relation &relation);
};


#endif//REWRITEHANDLER_H
