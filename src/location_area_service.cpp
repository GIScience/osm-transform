#include "location_area_service.h"

#include <boost/tokenizer.hpp>
#include <filesystem>
#include <fstream>
#include <gdal_priv.h>
#include <iostream>

namespace fs = std::filesystem;

const std::string LocationAreaService::delim_str_ = ";";

inline bool geo_col_check(std::string &data, std::string &geo_type) {
    if (geo_type == "wkt") {
        return data.starts_with("MULTIPOLYGON") || data.starts_with("POLYGON");
    }
    if (geo_type == "geojson") {
        return data.starts_with("{") && data.ends_with("}");
    }
    std::cout << "ERROR: invalid geometry type!" << std::endl;
    return false;
}

inline std::vector<std::string> split_str(std::string &line, const std::string &delim) {
    std::vector<std::string> tokens;
    size_t pos;
    while ((pos = line.find(delim)) != std::string::npos) {
        tokens.push_back(line.substr(0, pos));
        line.erase(0, pos + delim.length());
    }
    tokens.push_back(line);
    return tokens;
}

void LocationAreaService::load(const std::string &path) {
    std::cout << "Load area mapping..." << std::endl;

    auto area_file_path = processed_file_prefix_ + "area.csv";
    auto index_file_path = processed_file_prefix_ + "index.csv";
    auto id_file_path = processed_file_prefix_ + "id.csv";

    if (std::filesystem::exists(area_file_path) && std::filesystem::exists(id_file_path) && std::filesystem::exists(index_file_path)) {
        std::string l;
        std::ifstream area_file(area_file_path.c_str());
        if (area_file.is_open()) {
            while (getline(area_file, l)) {
                auto row = split_str(l, delim_str_);
                OGRGeometry *poGeom;
                OGRErr eErr = OGRERR_NONE;
                eErr = OGRGeometryFactory::createFromWkt(row[2].c_str(), nullptr, &poGeom);
                if (eErr != OGRERR_NONE) {
                    std::cout << "WARNING: processed area mapping file is corrupted!" << std::endl;
                    continue;
                }
                auto *poBBox = new OGREnvelope();
                poGeom->getEnvelope(poBBox);
                mapping_area_.insert({std::stoi(row[0]), AreaIntersect{static_cast<area_id_t>(std::stoi(row[1])), poGeom, poBBox}});
            }
            area_file.close();
        }
        std::ifstream index_file(index_file_path.c_str());
        if (index_file.is_open()) {
            while (getline(index_file, l)) {
                auto row = split_str(l, delim_str_);
                mapping_index_[std::stoi(row[0])] = std::stoi(row[1]);
            }
            index_file.close();
        }
        std::ifstream id_file(id_file_path.c_str());
        if (id_file.is_open()) {
            while (getline(id_file, l)) {
                auto row = split_str(l, delim_str_);
                mapping_id_[std::stoi(row[0])] = row[1];
            }
            id_file.close();
        }
        std::cout << "Successfully loaded from previously processed area mappings." << std::endl;
        output_mapping();
        initialized_ = true;
        return;
    }

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

    std::cout << "Save processed area mapping" << std::endl;
    std::ofstream o_area_file(area_file_path);
    if (o_area_file.is_open()) {
        for (const auto &[k, a]: mapping_area_) {
            o_area_file << k << delim_str_ << a.id << delim_str_ << a.geo->exportToWkt(OGRWktOptions(), nullptr) << std::endl;
        }
        o_area_file.close();
    }
    std::ofstream o_id_file(id_file_path);
    if (o_id_file.is_open()) {
        for (const auto &[k, a]: mapping_id_) {
            o_id_file << k << delim_str_ << a << std::endl;
        }
        o_id_file.close();
    }
    std::ofstream o_index_file(index_file_path);
    if (o_index_file.is_open()) {
        for (auto k = 0; auto a: mapping_index_) {
            if (a != 0) {
                o_index_file << k << delim_str_ << a << std::endl;
            }
            k++;
        }
        o_index_file.close();
    }

    output_mapping();
    if (valid_rows > 0) {
        std::cout << "Areas indexed: " << valid_rows << std::endl;
        initialized_ = true;
    }
}

void LocationAreaService::output_mapping() {
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
    std::cout << "Areas: " << mapping_id_.size() << ", Split geometries: " << split_geos_count << ", Grid: [ empty: " << no_area_count << ", single: " << single_area_count << ", multiple: " << multiple_area_count << " ] " << std::endl;
}

void LocationAreaService::add_area_to_mapping_index(area_id_t id, const std::string &geometry) {
    OGRGeometry *countryGeom;
    OGRErr eErr = OGRERR_NONE;
    if (geo_type_ == "wkt") {
        eErr = OGRGeometryFactory::createFromWkt(geometry.c_str(), nullptr, &countryGeom);
    } else if (geo_type_ == "geojson") {
        countryGeom = OGRGeometryFactory::createFromGeoJson(geometry.c_str());
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
        std::cout << "Processing area " << id << ", valid: " << countryGeom->IsValid();
    }
    std::uint32_t intersecting_grid_tiles = 0;
    std::uint32_t contained_grid_tiles = 0;
    OGREnvelope countryBBox, e;
    countryGeom->getEnvelope(&countryBBox);
    for (grid_id_t i = 0; i < grid_size_; i++) {
        OGRPolygon g = grid_[i];
        g.getEnvelope(&e);
        if (e.Intersects(countryBBox) && g.Intersects(countryGeom)) {
            intersecting_grid_tiles++;
            if (countryGeom->Contains(&g)) {
                contained_grid_tiles++;
                mapping_index_[i] = id;
            } else {
                mapping_index_[i] = area_id_multiple_;
                OGRGeometry *poGeom = countryGeom->Intersection(&g);
                auto *poBBox = new OGREnvelope();
                poGeom->getEnvelope(poBBox);
                mapping_area_.insert({i, AreaIntersect{id, poGeom, poBBox}});
            }
        }
    }
    if (debug_mode_) {
        std::cout << " => intersecting grid tiles: " << intersecting_grid_tiles << ", contained grid tiles: " << contained_grid_tiles << std::endl;
    }
    OGRGeometryFactory::destroyGeometry(countryGeom);
}

std::vector<std::string> LocationAreaService::get_area(osmium::Location l) {
    areaCheckCounter++;
    std::vector<std::string> areas;
    if (!initialized_) {
        return areas;
    }
    grid_id_t grid_index = ((int) (l.lat()*2) + 90*2) * 360*2 + ((int) (l.lon()*2) + 180*2);
    OGRPoint point(l.lon(), l.lat());
    if (debug_mode_) {
        std::cout << "Lookup point: (" << l.lon() << " " << l.lat() << ") grid index " << grid_index << " => " << mapping_index_[grid_index] << std::endl;
    }
    switch (mapping_index_[grid_index]) {
        case 0:// no area
            break;
        default:// single area
            areas.push_back(mapping_id_[mapping_index_[grid_index]]);
            break;
        case area_id_multiple_:// multiple areas
            auto range = mapping_area_.equal_range(grid_index);
            for (auto i = range.first; i != range.second; ++i) {
                bBoxCheckCounter++;
                OGREnvelope *env = i->second.env;
                if (env->MinX <= point.getX() &&
                    env->MinY <= point.getY() &&
                    env->MaxX >= point.getX() &&
                    env->MaxY >= point.getY()) {
                    geomCheckCounter++;
                    if (i->second.geo->Contains(&point)) {
                        areas.push_back(mapping_id_[i->second.id]);
                    }
                }
            }
            break;
    }
    if (debug_mode_) {
        std::cout << "Result: ";
        for (auto const &area: areas) {
            std::cout << area;
        }
        std::cout << std::endl;
    }
    return areas;
}

void LocationAreaService::printAreaMappingStats() const {
    std::cout << "Area mapping stats ";
    std::cout << "[ areaChecks: " << areaCheckCounter;
    std::cout << ", bBoxChecks: " << bBoxCheckCounter;
    std::cout << ", geomChecks: " << geomCheckCounter;
    std::cout << "]" << std::endl << std::flush;
}

LocationAreaService::LocationAreaService(bool debug_mode, std::uint16_t id_col, std::uint16_t geo_col, std::string &geo_type, bool file_has_header, std::string &processed_file_prefix) : debug_mode_(debug_mode), id_col_(id_col), geo_col_(geo_col), geo_type_(geo_type), file_has_header_(file_has_header), processed_file_prefix_(processed_file_prefix) {
    GDALAllRegister();
    grid_ = new OGRPolygon[grid_size_];
    for (area_id_t grid_lat = 0; grid_lat < 180*2; grid_lat++) {
        for (area_id_t grid_lon = 0; grid_lon < 360*2; grid_lon++) {
            grid_id_t grid_index = grid_lat * 360*2 + grid_lon;
            double box_lon = (double) grid_lon / 2. - 180;
            double box_lat = (double) grid_lat / 2. - 90;
            OGRLinearRing ring;
            OGRPolygon poly;
            ring.addPoint(box_lon, box_lat);
            ring.addPoint(box_lon + 0.5, box_lat);
            ring.addPoint(box_lon + 0.5, box_lat + 0.5);
            ring.addPoint(box_lon, box_lat + 0.5);
            ring.addPoint(box_lon, box_lat);
            ring.closeRings();
            poly.addRing(&ring);
            grid_[grid_index] = poly;
        }
    }
}
