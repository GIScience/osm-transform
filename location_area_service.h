#ifndef OSM_TRANSFORM_LOCATION_AREA_SERVICE_H
#define OSM_TRANSFORM_LOCATION_AREA_SERVICE_H

#include <limits>
#include <iostream>
#include <osmium/osm/location.hpp>
#include <ogr_geometry.h>

struct AreaIntersect {
    short id;
    OGRGeometry *geo;
};

class LocationAreaService {

private:
    typedef unsigned short area_id_t;
    static const int grid_size_ = 64800;
    static const int area_id_multiple_ = std::numeric_limits<area_id_t>::max();

    OGRPolygon grid_[grid_size_];
    area_id_t mapping_index_[grid_size_] = {0};
    std::multimap<unsigned int, AreaIntersect> mapping_area_;
    std::unordered_map<area_id_t, std::string> mapping_id_;
    std::unordered_map<area_id_t, std::string> mapping_name_;

    bool debug_mode_ = false;
    bool initialized_ = false;

    void add_area_to_mapping_index(short id, const std::string& geometry) {
        OGRGeometry *poGeom;
        OGRErr eErr = OGRERR_NONE;
        eErr = OGRGeometryFactory::createFromWkt(geometry.c_str(), nullptr, &poGeom);
        if (eErr != OGRERR_NONE) {
            std::string pszMessage;
            switch (eErr) {
                case OGRERR_NOT_ENOUGH_DATA:
                    pszMessage = "Not enough data to deserialize";
                    break;
                case OGRERR_UNSUPPORTED_GEOMETRY_TYPE:
                    pszMessage = "Unsupported geometry type";
                    break;
                case OGRERR_CORRUPT_DATA:
                    pszMessage = "Corrupt data";
                    break;
                default:
                    pszMessage = "Unrecognized error";
            }
            std::cout << "WARNING: CSV contains row with invalid geometry data: " << pszMessage << std::endl;
        } else {
            if (debug_mode_) {
                std::cout << "Processing area " << id << ", valid: " << poGeom->IsValid();
            }
            int intersecting_grid_tiles = 0;
            int contained_grid_tiles = 0;
            for (int i = 0; i < grid_size_; i++) {
                OGRPolygon e = grid_[i];

                if (e.Intersects(poGeom)) {
                    intersecting_grid_tiles++;
                    if (poGeom->Contains(&e)) {
                        contained_grid_tiles++;
                        mapping_index_[i] = id;
                    } else {
                        mapping_index_[i] = area_id_multiple_;
                        mapping_area_.insert({i, AreaIntersect{id, poGeom->Intersection(&e)}});
                    }
                }
            }
            if (debug_mode_) {
                std::cout << " => intersecting grid tiles: " << intersecting_grid_tiles << ", contained grid tiles: " << contained_grid_tiles << std::endl;
            }
        }
    }


public:
    explicit LocationAreaService(bool debug_mode);

    void load(const std::string path);

    std::vector<std::string> get_area(osmium::Location l);

    bool is_initialized() {
        return initialized_;
    }
};


#endif//OSM_TRANSFORM_LOCATION_AREA_SERVICE_H
