#include "LocationElevationService.h"

#include <filesystem>

#include "GeoTiff.h"

namespace fs = std::filesystem;

namespace bg = boost::geometry;
namespace bgm = bg::model;
namespace bgi = bg::index;

typedef bgm::point<double, 2, bg::cs::geographic<bg::degree>> point;
typedef bgm::box<point> box;
typedef std::pair<box, prioAndFileName> rTreeEntry;

inline auto sortRTreeEntryByPrio(const rTreeEntry &a, const rTreeEntry &b) { return a.second.prio < b.second.prio; }

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
        std::vector<std::string> geotiffs;
        for (auto &p: fs::recursive_directory_iterator(path)) {
            auto ext = p.path().extension().string();
            if (!boost::iequals(ext, ".tif") && !boost::iequals(ext, ".tiff") && !boost::iequals(ext, ".gtiff")) { continue; }
            const std::string filename{p.path().string()};
            geotiffs.push_back(filename);
        }
        auto maxStepWidth = 0.0;
        std::cout << "Load geotiff index...\n";
        osmium::ProgressBar pTiffs{geotiffs.size(), osmium::isatty(2)};
        auto loaded = 0;
        for (const auto& geotiff: geotiffs) {
            const auto tif = GDALDatasetUniquePtr(GDALDataset::FromHandle(GDALOpen(geotiff.c_str(), GA_ReadOnly)));

            auto reference = GeoTiff::getSpatialReference(tif->GetProjectionRef());
            const auto transformation = OGRCreateCoordinateTransformation(&reference, &WGS84);

            double transform[6] = {};
            tif->GetGeoTransform(transform);

            const double lngMin = transform[0] + 0 * transform[1] + 0 * transform[2];
            const double latMax = transform[3] + 0 * transform[4] + 0 * transform[5];
            const double lngMax = lngMin + tif->GetRasterXSize() * transform[1] + tif->GetRasterXSize() * transform[2];
            const double latMin = latMax + tif->GetRasterYSize() * transform[4] + tif->GetRasterYSize() * transform[5];

            double lng[2] = {lngMin, lngMax};
            double lat[2] = {latMin, latMax};
            transformation->Transform(2, lng, lat);

            box b(point(lng[0], lat[0]), point(lng[1], lat[1]));
            double lngStep = (lng[1] - lng[0]) / static_cast<double>(tif->GetRasterXSize());
            double latStep = (lat[1] - lat[0]) / static_cast<double>(tif->GetRasterYSize());
            auto prio = std::min(lngStep, latStep);
            maxStepWidth = std::max(prio, maxStepWidth);

            auto v = std::make_pair(b, prioAndFileName{prio, geotiff});
            //        std::cout << std::fixed << " insert = " << bg::wkt<box>(v.first) << " - " << v.second.prio << " - " << v.second.fileName << std::endl;
            rtree.insert(v);
            loaded += 1;
            pTiffs.update(loaded);
        }
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
LocationElevationService::LocationElevationService(uint cache_size) : m_cache_size(cache_size) {
    GDALAllRegister();
}
