#ifndef REWRITEHANDLER_H
#define REWRITEHANDLER_H

#include <filesystem>
#include <iostream>

#include <boost/regex.hpp>

#include <osmium/builder/osm_object_builder.hpp>
#include <osmium/handler.hpp>
#include <osmium/index/id_set.hpp>
#include <osmium/index/map/all.hpp>
#include <osmium/index/node_locations_map.hpp>
#include <osmium/index/nwr_array.hpp>
#include <osmium/memory/buffer.hpp>

#include "geotiff.h"
#include "location_elevation_service.h"
#include "location_area_service.h"

class RewriteHandler : public osmium::handler::Handler {

    osmium::memory::Buffer *buffer_;
    osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids_;
    osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> &no_elevation_;

    boost::regex &remove_tags_;
    const boost::regex non_digit_regex_ = boost::regex("[^0-9.]");

    osmium::memory::Buffer *node_buffer_;
    osmium::object_id_type next_node_id_;
    std::unique_ptr<osmium::index::map::Map<osmium::unsigned_object_id_type, osmium::Location>> &location_index_;
    LocationElevationService &location_elevation_;
    LocationAreaService &location_area_;
    bool interpolate_;
    double interpolate_threshold_;

    void copy_tags(osmium::builder::Builder &parent, const osmium::TagList &tags, double ele, const std::vector<std::string>& countries);
    void copy_tags(osmium::builder::Builder &parent, const osmium::TagList &tags);

    auto get_node_location(const osmium::object_id_type id) -> osmium::Location {
        return location_index_->get_noexcept(static_cast<osmium::unsigned_object_id_type>(id));
    }

    void add_refs(const osmium::Way &way, osmium::builder::Builder &builder);

    void interpolate(const osmium::Way &way, osmium::builder::WayNodeListBuilder &wnl_builder);

    void newNode(osmium::object_id_type id, LocationElevation &le);

public:
    unsigned long long processed_elements_ = 0;
    unsigned long long total_tags_ = 0;
    unsigned long long valid_tags_ = 0;
    bool add_elevation_ = false;
    unsigned long long nodes_with_elevation_ = 0;
    unsigned long long nodes_with_elevation_not_found_ = 0;
    unsigned long long nodes_added_by_interpolation_ = 0;
    unsigned long long nodes_with_single_country_ = 0;
    unsigned long long nodes_with_multiple_countries_ = 0;
    unsigned long long nodes_with_no_country_ = 0;

    explicit RewriteHandler(const osmium::object_id_type next_node_id,
                            std::unique_ptr<osmium::index::map::Map<osmium::unsigned_object_id_type, osmium::Location>> &location_index,
                            LocationElevationService &elevation_service,
                            LocationAreaService &area_service,
                            boost::regex &remove_tags,
                            osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids,
                            osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> &no_elevation,
                            bool interpolate,
                            double interpolate_threshold
                            ) : next_node_id_(next_node_id),
                                location_index_(location_index),
                                location_elevation_(elevation_service),
                                location_area_(area_service),
                                remove_tags_(remove_tags),
                                valid_ids_(valid_ids),
                                no_elevation_(no_elevation),
                                interpolate_(interpolate),
                                interpolate_threshold_(interpolate_threshold) {
    }

    void set_buffers(osmium::memory::Buffer *output_buffer, osmium::memory::Buffer *output_node_buffer) {
        buffer_ = output_buffer;
        node_buffer_ = output_node_buffer;
        processed_elements_ = 0;
        total_tags_ = 0;
        valid_tags_ = 0;
    }

    void node(const osmium::Node &node);

    void way(const osmium::Way &way);

    void relation(const osmium::Relation &relation);

    void printCountryStats() {
        std::cout << "Nodes with no country: " << nodes_with_no_country_ << ", "
                  << "single country: " << nodes_with_single_country_ << ", "
                  << "multiple countries: " << nodes_with_multiple_countries_
                  << std::endl;
    };
};


#endif//REWRITEHANDLER_H
