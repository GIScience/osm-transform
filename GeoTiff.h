#ifndef GEOTIFF_H
#define GEOTIFF_H
#include "utils.h"

#include <gdal_priv.h>
#include <iostream>
#include <filesystem>


class GeoTiff {
    GDALDatasetUniquePtr dataSet;
    OGRCoordinateTransformation *transformation;
    double transform[6] = {};
    int rasterHasNoData = 0;
    double rasterNoDataValue = 0.0;

public:
    explicit GeoTiff(const char *filename) {
        dataSet = GDALDatasetUniquePtr(GDALDataset::FromHandle(GDALOpenShared(filename, GA_ReadOnly)));
        if (dataSet == nullptr) return;
        const auto reference = getSpatialReference(dataSet->GetProjectionRef());
        transformation = OGRCreateCoordinateTransformation(&WGS84, &reference);
        dataSet->GetGeoTransform(transform);
        rasterNoDataValue = dataSet->GetRasterBand(1)->GetNoDataValue(&rasterHasNoData);
    }


    double elevation(double lng, double lat) const {
        transformation->Transform(1, &lng, &lat);
        auto x = static_cast<int>(floor((lng - transform[0]) / transform[1]));
        auto y = static_cast<int>(floor((lat - transform[3]) / transform[5]));
        const auto maxX = dataSet->GetRasterXSize();
        const auto maxY = dataSet->GetRasterYSize();
        if (x < -1 || y < -1 || x > maxX || y > maxY) {
            cout << "Coordinate out of bounds: POINT (" << lat << " " << lng << ")" << endl;
            return NO_DATA_VALUE;
        }

        // for some coordinates close to the borders of the tile space the transformation returns invalid coordinates,
        // because the tiles of the dataset are not cut along full degree lines.
        x = max(min(x, dataSet->GetRasterXSize()), 0);
        y = max(min(y, dataSet->GetRasterYSize()), 0);
        double pixel[2];
        if (dataSet->GetRasterBand(1)->RasterIO(GF_Read, x, y, 1, 1, pixel, 1, 1, GDT_CFloat64, 0, 0) != CE_None ||
            (rasterHasNoData && pixel[0] <= rasterNoDataValue)) { return NO_DATA_VALUE; }
        return pixel[0];
    }

    auto GetDescription() const { return dataSet->GetDriver()->GetDescription(); }

    auto GetRasterXSize() const { return dataSet->GetRasterXSize(); };
    auto GetRasterYSize() const { return dataSet->GetRasterYSize(); };
    auto GetRasterCount() const { return dataSet->GetRasterCount(); };
};


struct prioAndFileName {
    double prio;
    string fileName;
};

namespace fs = std::filesystem;

namespace bg = boost::geometry;
namespace bgm = bg::model;
namespace bgi = bg::index;

typedef bgm::point<double, 2, bg::cs::geographic<bg::degree>> point;
typedef bgm::box<point> box;
typedef std::pair<box, prioAndFileName> rTreeEntry;


inline auto sortRTreeEntryByPrio(const rTreeEntry &a, const rTreeEntry &b) { return a.second.prio < b.second.prio; };

inline auto generate_geo_tiff_index(bgi::rtree<rTreeEntry, bgi::quadratic<16>> &rtree, const std::string &path) {
    std::vector<string> geotiffs;
    for (auto &p: fs::recursive_directory_iterator(path)) {
        auto ext = p.path().extension().string();
        if (!boost::iequals(ext, ".tif") && !boost::iequals(ext, ".tiff") && !boost::iequals(ext, ".gtiff")) { continue; }
        const std::string filename{p.path().string()};
        geotiffs.push_back(filename);
    }
    auto maxStepWidth = 0.0;
    osmium::ProgressBar pTiffs{geotiffs.size(), osmium::isatty(2)};
    for (const auto& geotiff: geotiffs) {
        const auto tif = GDALDatasetUniquePtr(GDALDataset::FromHandle(GDALOpen(geotiff.c_str(), GA_ReadOnly)));

        auto reference = getSpatialReference(tif->GetProjectionRef());
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
        pTiffs.update(1);
    }
    return maxStepWidth;
}

#endif //GEOTIFF_H
