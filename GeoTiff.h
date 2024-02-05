#ifndef GEOTIFF_H
#define GEOTIFF_H

#include <boost/algorithm/string/predicate.hpp>
#include <boost/geometry.hpp>
#include <boost/geometry/geometries/box.hpp>
#include <boost/geometry/geometries/point.hpp>
#include <boost/geometry/index/rtree.hpp>

#include <filesystem>
#include <gdal_priv.h>
#include <iostream>
#include <osmium/util/file.hpp>
#include <osmium/util/progress_bar.hpp>

static constexpr double NO_DATA_VALUE = -32768.0;

static const OGRSpatialReference getWGS84Reference() {
    OGRSpatialReference reference;
    reference.SetWellKnownGeogCS("WGS84");
    reference.SetAxisMappingStrategy(OAMS_TRADITIONAL_GIS_ORDER);
    return reference;
}

static auto WGS84 = getWGS84Reference();

class GeoTiff {
    GDALDatasetUniquePtr dataSet;
    OGRCoordinateTransformation *transformation;
    double transform[6] = {};
    int rasterHasNoData = 0;
    double rasterNoDataValue = 0.0;

public:

    static auto getSpatialReference(const char *crs) {
        OGRSpatialReference reference;
        reference.importFromWkt(crs);
        reference.SetAxisMappingStrategy(OAMS_TRADITIONAL_GIS_ORDER);
        return reference;
    }

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
            std::cout << "Coordinate out of bounds: POINT (" << lat << " " << lng << ")\n";
            return NO_DATA_VALUE;
        }

        // for some coordinates close to the borders of the tile space the transformation returns invalid coordinates,
        // because the tiles of the dataset are not cut along full degree lines.
        x = std::max(std::min(x, dataSet->GetRasterXSize() - 1), 0);
        y = std::max(std::min(y, dataSet->GetRasterYSize() - 1), 0);
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

#endif //GEOTIFF_H
