#include "LocationElevationService.h"

#include "GeoTiff.h"

std::vector<LocationElevation> LocationElevationService::interpolate(osmium::Location from, osmium::Location to) {
    std::vector<LocationElevation> data;

    std::vector<rTreeEntry> result_s;
  //  rtree.query(bgi::contains(point(from.lon(),from.lat())), std::back_inserter(result_s));
    std::sort(result_s.begin(), result_s.end(), sortRTreeEntryByPrio);
    if (result_s.empty()) {
        return data;
    }

    auto entry = result_s.front();
    auto stepWidth = entry.second.prio;
    auto filename = entry.second.fileName;

    GeoTiff geo_tiff(filename.c_str());

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
        double ele = geo_tiff.elevation(lng, lat);
        data.emplace_back(LocationElevation(osmium::Location(lng, lat), ele));
    }
    return data;
}


void LocationElevationService::load(const std::string &path) {
    generate_geo_tiff_index(rtree, path);
}
