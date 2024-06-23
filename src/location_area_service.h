#ifndef OSM_TRANSFORM_LOCATION_AREA_SERVICE_H
#define OSM_TRANSFORM_LOCATION_AREA_SERVICE_H

#include <limits>
#include <iostream>
#include <osmium/osm/location.hpp>
#include <ogr_geometry.h>

typedef std::uint16_t area_id_t;
typedef std::uint32_t grid_id_t;

struct AreaIntersect {
    area_id_t id;
    OGRGeometry *geo;
    OGREnvelope *env;
};

class LocationAreaService {

private:
    static const grid_id_t grid_size_ = 259200;
    static const area_id_t area_id_multiple_ = std::numeric_limits<area_id_t>::max();
    static const std::string delim_str_;

    OGRPolygon *grid_;
    area_id_t mapping_index_[grid_size_] = {0};
    std::multimap<grid_id_t, AreaIntersect> mapping_area_;
    std::unordered_map<area_id_t, std::string> mapping_id_;

    std::uint16_t id_col_;
    std::uint16_t geo_col_;
    std::string geo_type_;
    std::string processed_file_prefix_;
    bool file_has_header_ = false;

    bool debug_mode_ = false;
    bool initialized_ = false;

    std::uint32_t areaCheckCounter = 0, geomCheckCounter = 0, bBoxCheckCounter = 0;

    void add_area_to_mapping_index(area_id_t id, const std::string& geometry);

    void output_mapping();

public:
    explicit LocationAreaService(bool debug_mode, std::uint16_t id_col, std::uint16_t geo_col, std::string& geo_type, bool file_has_header, std::string& processed_file_prefix);

    void load(const std::string& path);

    std::vector<std::string> get_area(osmium::Location l);

    bool is_initialized() {
        return initialized_;
    }

    void printAreaMappingStats() const;
};


#endif//OSM_TRANSFORM_LOCATION_AREA_SERVICE_H
