#include "location_area_service.h"

#include <boost/tokenizer.hpp>
#include <filesystem>
#include <fstream>
#include <gdal_priv.h>
#include <iostream>

namespace fs = std::filesystem;

void LocationAreaService::load(const std::string path) {
    std::cout << "Load area mapping ..." << std::endl;

    std::ifstream in(path.c_str());
    if (!in.is_open()) {
        std::cout << "Failed to open area mapping file!" << std::endl;
        return;
    }

    typedef boost::tokenizer<boost::escaped_list_separator<char>, std::string::const_iterator, std::string> Tokenizer;
    boost::escaped_list_separator<char> seps('\\', ';', '\"');
    std::vector<std::string> row;
    std::string line;

    int index = 0;
    int valid_rows = 0;
    mapping_id_[index] = "";
    mapping_name_[index] = "NO MAPPING";
    while (getline(in, line)) {
        Tokenizer tok(line, seps);
        row.assign(tok.begin(), tok.end());
        if (row.size() == 3) {
            if (row[2].starts_with("MULTIPOLYGON")) {
                valid_rows++;
                mapping_id_[index] = row[0];
                mapping_name_[index] = row[1];
                add_area_to_mapping_index(index, row[2]);
            } else {
                if (index > 0) {
                    std::cout << "WARNING: CSV contains row with invalid value in geometry column : " << row[2] << std::endl;
                }
            }
        } else {
            std::cout << "WARNING: CSV contains row with incorrect number of columns!" << std::endl;
        }
        index++;
    }
    if (debug_mode_) {
        int no_area_count = 0;
        int single_area_count = 0;
        int multiple_area_count = 0;
        int split_geos_count = 0;
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

std::vector<std::string> LocationAreaService::get_area(osmium::Location l) {
    std::vector<std::string> areas;
    if (initialized_) {
        unsigned int grid_index = (int)(l.lat() + 90) * 360 + (int)(l.lon() + 180);
        OGRPoint point(l.lon(), l.lat());
        if (debug_mode_) {
            std::cout << "Lookup point: (" << l.lon() << " " << l.lat() << ") grid index " << grid_index << " => " << mapping_index_[grid_index] << std::endl;
        }
        switch (mapping_index_[grid_index]) {
            case 0: // no area
                break;
            default: // single area
                areas.push_back(mapping_name_[mapping_index_[grid_index]]);
                break;
            case area_id_multiple_: // multiple areas
                auto range = mapping_area_.equal_range(grid_index);
                for (auto i = range.first; i != range.second; ++i) {
                    if (i->second.geo->Contains(&point)) {
                        areas.push_back(mapping_name_[i->second.id]);
                    }
                }
                break;
        }
        if (debug_mode_) {
            std::cout << "Result: ";
            for (auto const area : areas) {
                std::cout << area;
            }
            std::cout << std::endl;
        }
    }
    return areas;
}

LocationAreaService::LocationAreaService(bool debug_mode) : debug_mode_(debug_mode) {
    GDALAllRegister();
    for (int grid_lat = 0; grid_lat < 180; grid_lat++) {
        for (int grid_lon = 0; grid_lon < 360; grid_lon++) {
            unsigned int grid_index = grid_lat * 360 + grid_lon;
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
