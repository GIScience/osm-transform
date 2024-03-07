#include "location_area_service.h"

#include <boost/tokenizer.hpp>
#include <filesystem>
#include <fstream>
#include <gdal_priv.h>
#include <iostream>

namespace fs = std::filesystem;

inline bool geo_col_check(std::string& data, std::string& geo_type) {
    if (geo_type == "wkt") {
        return data.starts_with("MULTIPOLYGON") || data.starts_with("POLYGON");
    }
    if (geo_type == "geojson") {
        return data.starts_with("{") && data.ends_with("}");
    }
    std::cout << "ERROR: invalid geometry type!" << std::endl;
    return false;
}

void LocationAreaService::load(const std::string& path) {
    std::cout << "Load area mapping..." << std::endl;

    std::ifstream in(path.c_str());
    if (!in.is_open()) {
        std::cout << "Failed to open area mapping file!" << std::endl;
        return;
    }

    typedef boost::tokenizer<boost::escaped_list_separator<char>, std::string::const_iterator, std::string> Tokenizer;
    boost::escaped_list_separator<char> seps('\\', ';', '\"');
    std::vector<std::string> row;
    std::string line;

    area_id_t index = 0;
    area_id_t valid_rows = 0;
    mapping_id_[index] = "";
    if (!file_has_header_) {
        index++;
    }
    while (getline(in, line)) {
        Tokenizer tok(line, seps);
        row.assign(tok.begin(), tok.end());
        if (row.size() > std::max(id_col_, geo_col_)) {
            if (geo_col_check(row[geo_col_], geo_type_)) {
                if (index == 0) {
                    std::cout << "WARNING: CSV seems to contain data in the first row though area_mapping_has_header is set to true!" << std::endl;
                    index++;
                }
                valid_rows++;
                mapping_id_[index] = row[id_col_];
                add_area_to_mapping_index(index, row[geo_col_]);
            } else {
                if (index > 0) {
                    std::cout << "WARNING: CSV contains row with invalid value in geometry column! Row number: " << index + 1 << "!" << std::endl;
                }
            }
        } else {
            std::cout << "WARNING: CSV contains row with incorrect number of columns!" << std::endl;
        }
        index++;
    }
    if (debug_mode_) {
        std::uint32_t no_area_count = 0;
        std::uint32_t single_area_count = 0;
        std::uint32_t multiple_area_count = 0;
        std::uint32_t split_geos_count = 0;
        for (const auto v: mapping_index_) {
            switch (v) {
                case 0:
                    no_area_count++;
                    break;
                default:
                    single_area_count++;
                    break;
                case area_id_multiple_:
                    multiple_area_count++;
                    break;
            }
        }
        for (const auto &[k, a]: mapping_area_) {
            //            std::cout << "area[" << k << "] = (" << a.id << ", " << a.geo << ") " << std::endl;
            split_geos_count++;
        }
        std::cout << "Grid [ NO AREA: " << no_area_count << " SINGLE: " << single_area_count << " MULTIPLE: " << multiple_area_count << " ] Split geometries: " << split_geos_count << std::endl;
    }
    if (valid_rows > 0) {
        std::cout << "Areas indexed: " << valid_rows << std::endl;
        initialized_ = true;
    }
}

void LocationAreaService::add_area_to_mapping_index(area_id_t id, const std::string& geometry) {
    OGRGeometry *poGeom;
    OGRErr eErr = OGRERR_NONE;
    if (geo_type_ == "wkt") {
        eErr = OGRGeometryFactory::createFromWkt(geometry.c_str(), nullptr, &poGeom);
    } else if (geo_type_ == "geojson") {
        poGeom = OGRGeometryFactory::createFromGeoJson(geometry.c_str());
    } else {
    }
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
        return;
    }
    if (debug_mode_) {
        std::cout << "Processing area " << id << ", valid: " << poGeom->IsValid();
    }
    std::uint32_t intersecting_grid_tiles = 0;
    std::uint32_t contained_grid_tiles = 0;
    for (grid_id_t i = 0; i < grid_size_; i++) {
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
    OGRGeometryFactory::destroyGeometry(poGeom);
}

std::vector<std::string> LocationAreaService::get_area(osmium::Location l) {
    std::vector<std::string> areas;
    if (!initialized_) {
        return areas;
    }
    grid_id_t grid_index = ((int)l.lat() + 90) * 360 + ((int)l.lon() + 180);
    OGRPoint point(l.lon(), l.lat());
    if (debug_mode_) {
        std::cout << "Lookup point: (" << l.lon() << " " << l.lat() << ") grid index " << grid_index << " => " << mapping_index_[grid_index] << std::endl;
    }
    switch (mapping_index_[grid_index]) {
        case 0: // no area
            break;
        default: // single area
            areas.push_back(mapping_id_[mapping_index_[grid_index]]);
            break;
        case area_id_multiple_: // multiple areas
            auto range = mapping_area_.equal_range(grid_index);
            for (auto i = range.first; i != range.second; ++i) {
                if (i->second.geo->Contains(&point)) {
                    areas.push_back(mapping_id_[i->second.id]);
                }
            }
            break;
    }
    if (debug_mode_) {
        std::cout << "Result: ";
        for (auto const& area : areas) {
            std::cout << area;
        }
        std::cout << std::endl;
    }
    return areas;
}

LocationAreaService::LocationAreaService(bool debug_mode, std::uint16_t id_col, std::uint16_t geo_col, std::string& geo_type, bool file_has_header) : debug_mode_(debug_mode), id_col_(id_col), geo_col_(geo_col), geo_type_(geo_type), file_has_header_(file_has_header) {
    GDALAllRegister();
    for (std::uint16_t grid_lat = 0; grid_lat < 180; grid_lat++) {
        for (std::uint16_t grid_lon = 0; grid_lon < 360; grid_lon++) {
            grid_id_t grid_index = grid_lat * 360 + grid_lon;
            int box_lon = grid_lon - 180;
            int box_lat = grid_lat - 90;
            OGRLinearRing ring;
            OGRPolygon poly;
            ring.addPoint(box_lon, box_lat);
            ring.addPoint(box_lon + 1, box_lat);
            ring.addPoint(box_lon + 1, box_lat + 1);
            ring.addPoint(box_lon, box_lat + 1);
            ring.addPoint(box_lon, box_lat);
            ring.closeRings();
            poly.addRing(&ring);
            grid_[grid_index] = poly;
        }
    }
}
