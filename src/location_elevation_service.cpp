#include "location_elevation_service.h"

#include <filesystem>

#include "geotiff.h"

namespace fs = std::filesystem;

namespace bg = boost::geometry;
namespace bgm = bg::model;
namespace bgi = bg::index;

typedef bgm::point<double, 2, bg::cs::geographic<bg::degree>> point;
typedef bgm::segment<point> segment;
typedef bgm::box<point> box;
typedef std::pair<box, PrioAndFilename> rtree_entry;

inline auto sortRTreeEntryByPrio(const rtree_entry &a, const rtree_entry &b) { return a.second.prio < b.second.prio; }

std::vector<LocationElevation> LocationElevationService::interpolate(osmium::Location from, osmium::Location to) {
    std::vector<LocationElevation> data;
    std::vector<rtree_entry> query_result;
    box bbox;
    bg::envelope(segment(point(from.lon(),from.lat()), point(to.lon(),to.lat())), bbox);
    rtree_.query(bgi::intersects(bbox), std::back_inserter(query_result));
    std::sort(query_result.begin(), query_result.end(), sortRTreeEntryByPrio);
    if (query_result.empty()) { // no tiles found on the whole edge
        return data;
    }
    auto step_width =  query_result.front().second.prio;

    auto delta_x = to.lon() - from.lon();
    auto delta_y = to.lat() - from.lat();
    auto length = std::sqrt(delta_x * delta_x + delta_y * delta_y);

    const auto nx = delta_x / length;
    const auto ny = delta_y / length;
    const auto sx = nx * step_width;
    const auto sy = ny * step_width;

    auto steps = static_cast<int>(delta_x / sx);
    for (auto s = 0; s <= steps; s++) {
        double lng = from.lon() + sx * s;
        double lat = from.lat() + sy * s;
        auto loc = osmium::Location(lng, lat);
        double ele = elevation(loc, false);
        data.push_back(LocationElevation {loc, ele});
    }
    data.push_back(LocationElevation {to, elevation(to, false)});
    return data;
}

inline void put_tiffs_in_dir(const std::string &path, std::vector<std::string> &geotiffs) {
    if (std::filesystem::is_regular_file(path)){
        geotiffs.push_back(path);
        return;
    }
    try {
        for (auto &p: fs::recursive_directory_iterator(path)) {
            auto ext = p.path().extension().string();
            if (!boost::iequals(ext, ".tif") && !boost::iequals(ext, ".tiff") && !boost::iequals(ext, ".gtiff")) { continue; }
            const std::string filename{p.path().string()};
            geotiffs.push_back(filename);
        }
    } catch (std::filesystem::filesystem_error const& ex) {
        std::cout << "WARNING: Failed to read geotiffs from " << path << ". This might lead to a lesser success rate when determining location elevations.\n";
    }
}

void LocationElevationService::load(const std::vector<std::string> &paths) {
    std::vector<std::string> geotiffs;
    for (const auto& path : paths) {
        put_tiffs_in_dir(path, geotiffs);
    }
    std::cout << "Load geotiff index...\n";
    osmium::ProgressBar pTiffs{geotiffs.size(), osmium::isatty(2)};
    auto loaded = 0;
    for (const auto& geotiff: geotiffs) {
        const auto tif = GDALDatasetUniquePtr(GDALDataset::FromHandle(GDALOpen(geotiff.c_str(), GA_ReadOnly)));

        auto reference = Geotiff::getSpatialReference(tif->GetProjectionRef());
        const auto transformation = OGRCreateCoordinateTransformation(&reference, &WGS84);

        double transform[6] = {};
        tif->GetGeoTransform(transform);

        const double lng_min = transform[0] + 0 * transform[1] + 0 * transform[2];
        const double lat_max = transform[3] + 0 * transform[4] + 0 * transform[5];
        const double lng_max = lng_min + tif->GetRasterXSize() * transform[1] + tif->GetRasterXSize() * transform[2];
        const double lat_min = lat_max + tif->GetRasterYSize() * transform[4] + tif->GetRasterYSize() * transform[5];

        double lng[2] = {lng_min, lng_max};
        double lat[2] = {lat_min, lat_max};
        transformation->Transform(2, lng, lat);

        box b(point(lng[0], lat[0]), point(lng[1], lat[1]));
        double lngStep = (lng[1] - lng[0]) / static_cast<double>(tif->GetRasterXSize());
        double latStep = (lat[1] - lat[0]) / static_cast<double>(tif->GetRasterYSize());
        const auto prio = std::min(lngStep, latStep);

        auto v = std::make_pair(b, PrioAndFilename{prio, geotiff});
        rtree_.insert(v);
        loaded += 1;
        pTiffs.update(loaded);
    }
    initialized_ = true;
    std::cout << std::endl << "geotiff tiles indexed: " << rtree_.size() << std::endl;
}

std::shared_ptr<Geotiff> LocationElevationService::load_tiff(const char * filename) {
    const auto search = cache_.find(filename);
    ulong filesize = 0;
    if (tile_size_.count(filename)) {
        filesize = tile_size_[filename];
    } else {
        filesize = std::filesystem::file_size(filename);
        tile_size_.insert(std::make_pair(filename, filesize));
    }

    if (search != cache_.end()) {
        const auto geoTiff = cache_.at(filename);
        lru_.remove(filename);
        lru_.emplace_front(filename);
        return geoTiff;
    }

    if (!std::filesystem::exists(filename)) {
        return nullptr;
    }
    auto geotiff = std::make_shared<Geotiff>(filename, debug_mode_);
    if (geotiff == nullptr) {
        return nullptr;
    }

    while (mem_size_ > 0 && mem_size_ + tile_size_[filename] > cache_limit_) {
        auto to_remove = lru_.back();
        mem_size_ -= tile_size_[to_remove];
        cache_.erase(to_remove);
        lru_.pop_back();
    }
    cache_.insert(make_pair(filename, geotiff));
    mem_size_ += tile_size_[filename];
    lru_.emplace_front(filename);

    if (debug_mode_) {
        printf("Dataset opened. (format: %s; size: %d x %d x %d, cache mem size: %lu / %lu)\n", geotiff->GetDescription(),
               geotiff->GetRasterXSize(), geotiff->GetRasterYSize(), geotiff->GetRasterCount(), mem_size_, cache_limit_);
    }
    return geotiff;
}

double LocationElevationService::elevation(osmium::Location l, bool count) {
    std::vector<rtree_entry> query_result;
    rtree_.query(bgi::contains(point(l.lon(),l.lat())), std::back_inserter(query_result));
    std::sort(query_result.begin(), query_result.end(), sortRTreeEntryByPrio);
    if (query_result.empty()) {
        return kNoDataValue;
    }
    auto filename = query_result.front().second.filename;
    auto geo_tiff = load_tiff(filename.c_str());
    double ele = geo_tiff->elevation(l.lon(), l.lat());

    if (ele != kNoDataValue && count) {
        if (filename.starts_with("srtm")) {
            found_srtm_++;
        } else if (filename.contains("gmted")) {
            found_gmted_++;
        } else {
            found_custom_++;
        }
    }
    return ele;
}

LocationElevationService::LocationElevationService(ulong cache_limit, bool debug_mode) : cache_limit_(cache_limit), debug_mode_(debug_mode) {
    GDALAllRegister();
}
