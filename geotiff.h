#ifndef GEOTIFF_H
#define GEOTIFF_H

#include <gdal_priv.h>
#include <iostream>
#include <osmium/util/file.hpp>
#include <osmium/util/progress_bar.hpp>

static constexpr double kNoDataValue = -32768.0;

static const OGRSpatialReference getWGS84Reference() {
    OGRSpatialReference reference;
    reference.SetWellKnownGeogCS("WGS84");
    reference.SetAxisMappingStrategy(OAMS_TRADITIONAL_GIS_ORDER);
    return reference;
}

static auto WGS84 = getWGS84Reference();

class Geotiff {
    GDALDatasetUniquePtr dataset_;
    OGRCoordinateTransformation *transformation_;
    double transform_[6] = {};
    int raster_has_no_data_ = 0;
    double raster_no_data_value_ = 0.0;

public:

    static auto getSpatialReference(const char *crs) {
        OGRSpatialReference reference;
        reference.importFromWkt(crs);
        reference.SetAxisMappingStrategy(OAMS_TRADITIONAL_GIS_ORDER);
        return reference;
    }

    explicit Geotiff(const char *filename) {
        dataset_ = GDALDatasetUniquePtr(GDALDataset::FromHandle(GDALOpenShared(filename, GA_ReadOnly)));
        if (dataset_ == nullptr) return;
        const auto reference = getSpatialReference(dataset_->GetProjectionRef());
        transformation_ = OGRCreateCoordinateTransformation(&WGS84, &reference);
        dataset_->GetGeoTransform(transform_);
        raster_no_data_value_ = dataset_->GetRasterBand(1)->GetNoDataValue(&raster_has_no_data_);
    }

    virtual ~Geotiff() {
        OGRCoordinateTransformation::DestroyCT(transformation_);
    }

    double elevation(double lng, double lat) const {
        transformation_->Transform(1, &lng, &lat);
        auto x = static_cast<int>(floor((lng - transform_[0]) / transform_[1]));
        auto y = static_cast<int>(floor((lat - transform_[3]) / transform_[5]));
        const auto max_x = dataset_->GetRasterXSize();
        const auto max_y = dataset_->GetRasterYSize();
        if (x < -1 || y < -1 || x > max_x || y > max_y) {
            std::cout << "Coordinate out of bounds: Image coordinates (" << x << ", " << y << ") POINT (" << lat << " " << lng << ")\n";
            return kNoDataValue;
        }

        // for some coordinates close to the borders of the tile space the transformation_ returns invalid coordinates,
        // because the tiles of the dataset are not cut along full degree lines.
        x = std::max(std::min(x, dataset_->GetRasterXSize() - 1), 0);
        y = std::max(std::min(y, dataset_->GetRasterYSize() - 1), 0);
        double pixel[2];
        if (dataset_->GetRasterBand(1)->RasterIO(GF_Read, x, y, 1, 1, pixel, 1, 1, GDT_CFloat64, 0, 0) != CE_None ||
            (raster_has_no_data_ && pixel[0] <= raster_no_data_value_)) { return kNoDataValue; }
        return pixel[0];
    }

    auto GetDescription() const { return dataset_->GetDriver()->GetDescription(); }

    auto GetRasterXSize() const { return dataset_->GetRasterXSize(); };
    auto GetRasterYSize() const { return dataset_->GetRasterYSize(); };
    auto GetRasterCount() const { return dataset_->GetRasterCount(); };

};

#endif //GEOTIFF_H
