#ifndef UTILS_H
#define UTILS_H
#include <fstream>
#include <ogr_spatialref.h>
#include <vector>
#include <chrono>

using namespace std;
typedef unsigned long long llu;

static bool file_exists(const string &filename) {
    const ifstream ifile(filename.c_str());
    return static_cast<bool>(ifile);
}

static llu filesize(const string &filename) {
    ifstream in(filename.c_str(), std::ifstream::ate | std::ifstream::binary);
    return (llu) in.tellg();
}

static string remove_extension(const string &filename) {
    const size_t lastdot = filename.find_first_of(".");
    if (lastdot == string::npos) return filename;
    return filename.substr(0, lastdot);
}

static std::string getTimeStr() {
    const std::time_t now = std::chrono::system_clock::to_time_t(std::chrono::system_clock::now());
    std::string s(30, '\0');
    std::strftime(&s[0], s.size(), "%Y-%m-%d %H:%M:%S", std::localtime(&now));
    return s;
}


static OGRSpatialReference getWGS84Reference() {
    OGRSpatialReference reference;
    reference.SetWellKnownGeogCS("WGS84");
    reference.SetAxisMappingStrategy(OAMS_TRADITIONAL_GIS_ORDER);
    return reference;
}

static auto WGS84 = getWGS84Reference();
static constexpr double NO_DATA_VALUE = -32768.0;


static OGRSpatialReference getSpatialReference(const char *crs) {
    OGRSpatialReference reference;
    reference.importFromWkt(crs);
    reference.SetAxisMappingStrategy(OAMS_TRADITIONAL_GIS_ORDER);
    return reference;
}

#endif //UTILS_H
