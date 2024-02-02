#include "LocationElevationService.h"

#include <filesystem>

#include "GeoTiff.h"

std::vector<LocationElevation> LocationElevationService::interpolate(osmium::Location from, osmium::Location to) {
    std::vector<LocationElevation> data;

    std::vector<rTreeEntry> result_s;
    rtree.query(bgi::contains(point(from.lon(),from.lat())), std::back_inserter(result_s));
    std::sort(result_s.begin(), result_s.end(), sortRTreeEntryByPrio);
    if (result_s.empty()) {
        return data;
    }

    auto entry = result_s.front();
    auto stepWidth = entry.second.prio;
    auto filename = entry.second.fileName;


    auto geo_tiff = load_tiff(filename.c_str());

    auto deltaX = to.lon() - from.lon();
    auto deltaY = to.lat() - from.lat();
    auto length = std::sqrt(deltaX * deltaX + deltaY * deltaY);

    const auto nX = deltaX / length;
    const auto nY = deltaY / length;
    const auto sX = nX * stepWidth;
    const auto sY = nY * stepWidth;

    auto steps = static_cast<int>(deltaX / sX);
    for (auto s = 1; s <= steps; s++) {
        double lng = from.lon() + sX * s;
        double lat = from.lat() + sY * s;
        double ele = geo_tiff->elevation(lng, lat);
        auto loc = osmium::Location(lng, lat);
        data.push_back(LocationElevation {loc, ele});
    }
    return data;
}


void LocationElevationService::load(const std::string &path) {
    generate_geo_tiff_index(rtree, path);
}
std::shared_ptr<GeoTiff> LocationElevationService::load_tiff(const char * filename) {
    const auto search = m_cache.find(filename);
    if (search != m_cache.end()) {
        const auto geoTiff = m_cache.at(filename);
        m_lru.remove(filename);
        m_lru.emplace_front(filename);
        return geoTiff;
    }

    if (!std::filesystem::exists(filename)) {
        return nullptr;
    }
    auto geoTiff = std::make_shared<GeoTiff>(filename);
    if (geoTiff == nullptr) {
        return nullptr;
    }
    m_cache.insert(make_pair(filename, geoTiff));
    if (m_lru.size() == m_cache_size) {
        m_cache.erase(m_lru.back());
        m_lru.pop_back();
    }

        printf("Dataset opened. (format: %s; size: %d x %d x %d)\n", geoTiff->GetDescription(),
               geoTiff->GetRasterXSize(), geoTiff->GetRasterYSize(), geoTiff->GetRasterCount());
    m_lru.emplace_front(filename);

    return geoTiff;
}
double LocationElevationService::elevation(osmium::Location l) {
    std::vector<rTreeEntry> result_s;
    rtree.query(bgi::contains(point(l.lon(),l.lat())), std::back_inserter(result_s));
    std::sort(result_s.begin(), result_s.end(), sortRTreeEntryByPrio);
    if (result_s.empty()) {
        return NO_DATA_VALUE;
    }

    auto entry = result_s.front();
    auto filename = entry.second.fileName;
    auto geo_tiff = load_tiff(filename.c_str());

    return geo_tiff->elevation(l.lon(), l.lat());
}
LocationElevationService::LocationElevationService() {
    GDALAllRegister();
}
