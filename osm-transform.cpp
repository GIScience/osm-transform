#include <string>
#include <fstream>
#include <iostream>
#include <chrono>
#include <ctime>
#include <unordered_map>
#include <limits>

#include <boost/regex.hpp>
#include <boost/filesystem.hpp>
#include <boost/unordered_set.hpp>
#include <libgen.h>
#include <libconfig.h++>

#include "gdal.h"
#include "gdal_priv.h"
#include "gdal_utils.h"
#include "cpl_conv.h"
#include "node-locations.hpp"

#include <osmium/io/any_input.hpp>
#include <osmium/io/any_output.hpp>
#include <osmium/handler.hpp>
#include <osmium/io/pbf_input.hpp>
#include <osmium/io/pbf_output.hpp>
#include <osmium/osm/object.hpp>
#include <osmium/osm/node.hpp>
#include <osmium/osm/way.hpp>
#include <osmium/osm/relation.hpp>
#include <osmium/util/file.hpp>
#include <osmium/util/progress_bar.hpp>

using namespace std;
typedef unsigned long long llu;
typedef vector<unsigned int> vi;
const int BITWIDTH_INT = std::numeric_limits<unsigned int>::digits;


void setBit(vi &A, const llu k) {
    if (k < 0) return;
    A[k / BITWIDTH_INT] |= 1 << (k % BITWIDTH_INT);  // Set the bit at the k-th position in A
}

bool testBit(vi &A, const llu k) {
    if (k < 0) return 0;
    return (A[k / BITWIDTH_INT] & (1 << (k % BITWIDTH_INT))) != 0;
}

llu countBits(vi &A) {
    llu count = 0;
    for (const auto &intval: A) {
        count += __builtin_popcount(intval);
    }
    return count;
}

bool file_exists(const string &filename) {
    const ifstream ifile(filename.c_str());
    return (bool) ifile;
}

llu filesize(const string &filename) {
    ifstream in(filename.c_str(), std::ifstream::ate | std::ifstream::binary);
    return (llu) in.tellg();
}

string remove_extension(const string &filename) {
    const size_t lastdot = filename.find_first_of(".");
    if (lastdot == string::npos) return filename;
    return filename.substr(0, lastdot);
}

std::string getTimeStr() {
    const std::time_t now = std::chrono::system_clock::to_time_t(std::chrono::system_clock::now());
    std::string s(30, '\0');
    std::strftime(&s[0], s.size(), "%Y-%m-%d %H:%M:%S", std::localtime(&now));
    return s;
}

static OGRSpatialReference getReference(const char * crs) {
    OGRSpatialReference reference;
    reference.SetWellKnownGeogCS(crs);
    reference.SetAxisMappingStrategy(OAMS_TRADITIONAL_GIS_ORDER);
    return reference;
}

static auto WGS84 = getReference("WGS84");
static const int NO_DATA_VALUE = -32768;

class MaxIDHandler : public osmium::handler::Handler {
public:
    llu node_max_id = 0;
    llu way_max_id = 0;
    llu relation_max_id = 0;

    void node(const osmium::Node &node) {
        if (node.id() < 0) return;
        node_max_id = node.id() > node_max_id ? node.id() : node_max_id;
    }

    void way(const osmium::Way &way) {
        if (way.id() < 0) return;
        way_max_id = way.id() > way_max_id ? way.id() : way_max_id;
        for (const auto &node_ref: way.nodes()) {
            node_max_id = node_ref.ref() > node_max_id ? node_ref.ref() : node_max_id;
        }
    }

    void relation(const osmium::Relation &rel) {
        if (rel.id() < 0) return;
        relation_max_id = rel.id() > relation_max_id ? rel.id() : relation_max_id;

        for (const auto &member: rel.members()) {
            if (member.type() == osmium::item_type::node) {
                node_max_id = member.ref() > node_max_id ? member.ref() : node_max_id;
            }
            if (member.type() == osmium::item_type::way) {
                way_max_id = member.ref() > way_max_id ? member.ref() : way_max_id;
            }
        }
    }
};

class FirstPassHandler : public osmium::handler::Handler {
    friend ostream &operator<<(ostream &out, const FirstPassHandler &ce);

    const set<string> invalidating_tags{"building", "landuse", "boundary", "natural", "place", "waterway", "aeroway",
                                        "aviation", "military", "power", "communication", "man_made"};
    // const set<string> invalidating_tags{"building", "landuse"};
    boost::regex *remove_tags;
    llu node_max_id = 0;
    llu way_max_id = 0;
    llu relation_max_id = 0;

    void exitSegfault(string type, const llu id) {
        printf("%s ID %lld exceeds the allocated flag memory. Please increase the value in the config file. \nTo determine the exact value required, run this tool with the -c option.\n",
               type.c_str(), id);
        exit(4);
    }

    bool validating_tags(const string &tag, const string &value) {
        if (tag == "highway") return true;
        if (tag == "route") return true;
        if (tag == "railway" && value == "platform") return true;
        if (tag == "public_transport" && value == "platform") return true;
        if (tag == "man_made" && value == "pier") return true;
        return false;
    }

    bool check_tags(const osmium::TagList &tags) {
        int tag_count = 0;
        bool is_removable = false;
        for (const osmium::Tag &tag: tags) {
            if (!boost::regex_match(tag.key(), *remove_tags)) {
                tag_count++;
                if (validating_tags(tag.key(), tag.value())) {
                    return false;
                } else if (invalidating_tags.count(tag.key()) > 0) {
                    is_removable = true;
                }
            }
        }
        return tag_count == 0 || is_removable;
    }

public:

    llu node_count = 0;
    llu relation_count = 0;
    llu way_count = 0;

    vi *valid_nodes;
    vi *valid_ways;
    vi *valid_relations;

    bool DEBUG_NO_FILTER = false;

    void init(boost::regex *re, vi *i_valid_nodes, vi *i_valid_ways, vi *i_valid_relations, const bool debug_no_filter,
              const llu i_node_max_id, const llu i_way_max_id, const llu i_relation_max_id) {
        remove_tags = re;
        valid_nodes = i_valid_nodes;
        valid_ways = i_valid_ways;
        valid_relations = i_valid_relations;
        DEBUG_NO_FILTER = debug_no_filter;
        node_max_id = i_node_max_id;
        way_max_id = i_way_max_id;
        relation_max_id = i_relation_max_id;
    }

    void node(const osmium::Node &node) {
        if (node.id() < 0) return;
        if (node.id() > node_max_id) {
            exitSegfault("Node", node.id());
        }
        node_count++;
    }

    void way(const osmium::Way &way) {
        if (way.id() < 0) return;
        if (way.id() > way_max_id) {
            exitSegfault("Way", way.id());
        }
        way_count++;
        if (DEBUG_NO_FILTER || way.id() < 0 || way.nodes().size() < 2 || check_tags(way.tags())) {
            return;
        }
        for (const osmium::NodeRef &n: way.nodes()) {
            setBit(*valid_nodes, n.ref());
        }
        setBit(*valid_ways, way.id());
    }

    void relation(const osmium::Relation &rel) {
        if (rel.id() < 0) return;
        if (rel.id() > relation_max_id) {
            exitSegfault("Relation", rel.id());
        }
        relation_count++;
        if (DEBUG_NO_FILTER || rel.id() < 0 || check_tags(rel.tags())) {
            return;
        }
        for (const auto &member: rel.members()) {
            if (member.type() == osmium::item_type::node) {
                setBit(*valid_nodes, member.ref());
            }
        }
        setBit(*valid_relations, rel.id());
    }
};

class RewriteHandler : public osmium::handler::Handler {
    friend ostream &operator<<(ostream &out, const RewriteHandler &ce);

    osmium::memory::Buffer *m_buffer;
    vi *valid_nodes;
    vi *valid_ways;
    vi *valid_relations;
    boost::regex *remove_tags;
    boost::regex non_digit_regex;
    bool DEBUG_NO_FILTER = false;
    bool DEBUG_NO_TAG_FILTER = false;


    unordered_map<string, GDALDataset *> elevationData;
    int cache_size = -1;
    list<string> cache_queue;
    ofstream *log;

    osmium::memory::Buffer *m_new_node_buffer;
    osmid_t m_next_node_id;
    shared_ptr<node_locations_t> m_cache;

    void copy_tags(osmium::builder::Builder &parent, const osmium::TagList &tags, const int ele = NO_DATA_VALUE) {
        osmium::builder::TagListBuilder builder{parent};
        for (const auto &tag: tags) {
            total_tags++;
            if (DEBUG_NO_TAG_FILTER || !boost::regex_match(tag.key(), *remove_tags)) {
                string key = tag.key();
                if (key == "ele") { // keep ele tags only if no ele value passed
                    if (ele == NO_DATA_VALUE) {
                        valid_tags++;
                        string tagval(tag.value());
                        const string empty = "";
                        string value = regex_replace(tagval, non_digit_regex, empty);
                        builder.add_tag("ele", value);
                    }
                } else {
                    valid_tags++;
                    builder.add_tag(tag);
                }
            }
        }
        if (ele > NO_DATA_VALUE) {
            builder.add_tag("ele", to_string(ele));
        }
    }

    double getElevationCGIAR(const double lat, const double lng, const bool debug = false) {
        const int lngIndex = floor(1 + (180 + lng) / 5);
        const int latIndex = floor(1 + (60 - lat) / 5);
        char pszFilename[100];
        snprintf(pszFilename, 24,"srtmdata/srtm_%02d_%02d.tif", lngIndex, latIndex);
        if (debug)
            printf("Filename for coordinates %.6f - %.6f : %s\n", lng, lat, pszFilename);
        return getElevationFromFile(lat, lng, pszFilename, debug);
    }

    double getElevationGMTED(const double lat, const double lng, const bool debug = false) {
        const int lngIndex = static_cast<int>(-180 + floor((180 + lng) / 30) * 30);
        const int latIndex = static_cast<int>(-70 + floor((70 + lat) / 20) * 20);
        const char lngPre = lngIndex < 0 ? 'W' : 'E';
        const char latPre = latIndex < 0 ? 'S' : 'N';
        char pszFilename[100];
        snprintf(pszFilename, 43, "gmteddata/%02d%c%03d%c_20101117_gmted_mea075.tif", abs(latIndex), latPre, abs(lngIndex),
                lngPre);
        if (debug)
            printf("Filename for coordinates %.6f - %.6f : %s\n", lng, lat, pszFilename);
        return getElevationFromFile(lat, lng, pszFilename, debug);
    }

    double getElevationFromFile(const double lat, const double lng, char *pszFilename, const bool debug = false) {
        GDALDataset *poDataset;
        const auto search = elevationData.find(pszFilename);
        if (search != elevationData.end()) {
            poDataset = elevationData.at(pszFilename);
            cache_queue.remove(pszFilename);
        } else {
            if (!file_exists(pszFilename)) {
                if (debug)
                    cout << "File does not exist: " << pszFilename << endl;
                return NO_DATA_VALUE;
            }
            poDataset = (GDALDataset *) GDALOpenShared(pszFilename, GA_ReadOnly);
            if (poDataset == nullptr) {
                if (debug)
                    cout << "Failed to read input data from file " << pszFilename << endl;
                return NO_DATA_VALUE;
            }
            elevationData.insert(make_pair(pszFilename, poDataset));
            if (cache_queue.size() == cache_size) {
                elevationData.erase(cache_queue.back());
                cache_queue.pop_back();
            }
            if (debug)
                printf("Dataset opened. (format: %s; size: %d x %d x %d)\n", poDataset->GetDriver()->GetDescription(),
                       poDataset->GetRasterXSize(), poDataset->GetRasterYSize(), poDataset->GetRasterCount());
        }
        cache_queue.push_front(pszFilename);

        double adfGeoTransform[6];
        double adfInvGeoTransform[6];
        if (poDataset->GetGeoTransform(adfGeoTransform) != CE_None) {
            if (debug)
                cout << "Failed to get transformation from input data." << endl;
            return NO_DATA_VALUE;
        }
        if (!GDALInvGeoTransform(adfGeoTransform, adfInvGeoTransform)) {
            if (debug)
                cout << "Failed to get reverse transformation." << endl;
            return NO_DATA_VALUE;
        }
        int iPixel = static_cast<int>(floor(
                adfInvGeoTransform[0] + adfInvGeoTransform[1] * lng + adfInvGeoTransform[2] * lat));
        int iLine = static_cast<int>(floor(
                adfInvGeoTransform[3] + adfInvGeoTransform[4] * lng + adfInvGeoTransform[5] * lat));

        // for some coordinates close to the borders of the tile space the transformation returns invalid coordinates,
        // because the tiles of the dataset are not cut along full degree lines.
        if (iPixel == poDataset->GetRasterXSize()) {
            iPixel = poDataset->GetRasterXSize() - 1;
        }
        if (iLine == poDataset->GetRasterYSize()) {
            iLine = poDataset->GetRasterYSize() - 1;
        }
        if (iPixel < 0) {
            iPixel = 0;
        }
        if (iLine < 0) {
            iLine = 0;
        }
        if (debug) {
            printf("Coordinates: %.7f %.7f\n", lat, lng);
            printf("Image coordinates: %d %d\n", iPixel, iLine);
        }

        double adfPixel[2];
        if (poDataset->GetRasterBand(1)->RasterIO(GF_Read, iPixel, iLine, 1, 1, adfPixel, 1, 1, GDT_CFloat64, 0, 0) !=
            CE_None) {
            if (debug) {
                cout << "Failed to read data at coordinates." << endl;
            }
            return NO_DATA_VALUE;
        }
        return adfPixel[0];
    }

public:

    llu valid_elements = 0;
    llu processed_elements = 0;
    llu total_tags = 0;
    llu valid_tags = 0;
    bool addElevation = false;
    bool overrideValues = false;
    llu nodes_with_elevation_srtm_precision = 0;
    llu nodes_with_elevation_gmted_precision = 0;
    llu nodes_with_elevation = 0;
    llu nodes_with_elevation_not_found = 0;

    explicit RewriteHandler(const osmid_t next_node_id) : m_next_node_id(next_node_id), m_cache(std::make_unique<node_locations_t>()) {}

    void set_buffer(osmium::memory::Buffer *buffer) {
        m_buffer = buffer;
        valid_elements = 0;
        processed_elements = 0;
        total_tags = 0;
        valid_tags = 0;
    }

    void set_new_node_buffer(osmium::memory::Buffer *buffer) {
        m_new_node_buffer = buffer;
    }

    void init(const int i_cache_size, boost::regex *re, vi *i_valid_nodes, vi *i_valid_ways, vi *i_valid_relations,
              ofstream *logref, const bool debug_no_filter, const bool debug_no_tag_filter) {
        cache_size = i_cache_size;
        remove_tags = re;
        valid_nodes = i_valid_nodes;
        valid_ways = i_valid_ways;
        valid_relations = i_valid_relations;
        valid_elements = valid_nodes->size() + valid_ways->size() + valid_relations->size();
        log = logref;
        DEBUG_NO_FILTER = debug_no_filter;
        DEBUG_NO_TAG_FILTER = debug_no_tag_filter;
        non_digit_regex = boost::regex("[^0-9.]");
    }

    void node(const osmium::Node &node) {
        processed_elements++;
        if (node.id() < 0) return;
        {
            if (DEBUG_NO_FILTER || testBit(*valid_nodes, node.id()) > 0) {
                osmium::builder::NodeBuilder builder{*m_buffer};
                builder.set_id(node.id());
                builder.set_location(node.location());
                int ele = NO_DATA_VALUE;
                if (addElevation) {
                    if (!overrideValues && node.tags().has_key("ele")) {
                        nodes_with_elevation++;
                    } else {
                        ele = getElevationCGIAR(node.location().lat(), node.location().lon());
                        if (ele != NO_DATA_VALUE) {
                            nodes_with_elevation_srtm_precision++;
                        } else {
                            ele = getElevationGMTED(node.location().lat(), node.location().lon());
                            if (ele != NO_DATA_VALUE) {
                                nodes_with_elevation_gmted_precision++;
                            } else {
                                nodes_with_elevation_not_found++;
                                *log << getTimeStr() << " ele retrieval failed: " << node.location().lat() << " "
                                        << node.location().lon() << endl;
                                ele = 0.0;// GH elevation code defaults to 0
                            }
                        }
                    }
                }
                copy_tags(builder, node.tags(), ele);
                m_cache->set(node.id(), node.location(), ele);
            }
        }
        m_buffer->commit();
    }

    void way(const osmium::Way &way) {
        processed_elements++;
        if (way.id() < 0) return;
        {
            if (DEBUG_NO_FILTER || testBit(*valid_ways, way.id()) > 0) {
                auto next_node_id = m_next_node_id;

                std::vector<NodeWithElevation> nodes(way.nodes().size());
                for (const auto &node: way.nodes()) {
                    nodes.push_back(m_cache->get(node.ref()));
                }

                // nodes = splitWay(nodes);

                {
                    osmium::builder::WayBuilder builder{*m_buffer};
                    builder.set_id(way.id());
                    copy_tags(builder, way.tags());
                    {
                        osmium::builder::WayNodeListBuilder wnl_builder{builder};
                        for (const auto &node: nodes) {
                            if (node.id() < 0) {
                                wnl_builder.add_node_ref(m_next_node_id++);
                            } else {
                                wnl_builder.add_node_ref(node.id());
                            }
                        }
                    }
                }
                for (const auto &node: nodes) {
                    {
                        osmium::builder::NodeBuilder nodeBuilder(*m_new_node_buffer);
                        nodeBuilder.set_id(next_node_id++);
                        nodeBuilder.set_location(osmium::Location(node.x(), node.y()));
                        osmium::builder::TagListBuilder nodeTagsBuilder{nodeBuilder};
                        nodeTagsBuilder.add_tag("ele", to_string(node.elevation()));
                    }
                    m_new_node_buffer->commit();
                }
            }
            m_buffer->commit();
        }
    }

    void relation(const osmium::Relation &relation) {
        processed_elements++;
        if (relation.id() < 0) return;
        {
            if (DEBUG_NO_FILTER || testBit(*valid_relations, relation.id()) > 0) {
                osmium::builder::RelationBuilder builder{*m_buffer};
                builder.set_id(relation.id());
                builder.add_item(relation.members());
                copy_tags(builder, relation.tags());
            }
        }
        m_buffer->commit();
    }
};

ostream &operator<<(ostream &out, const FirstPassHandler &handler) {
    return out << "valid nodes: " << countBits(*handler.valid_nodes) << " (" << handler.node_count << "), "
               << "valid ways: " << countBits(*handler.valid_ways) << " (" << handler.way_count << "), "
               << "valid relations: " << countBits(*handler.valid_relations) << " (" << handler.relation_count << ")";
}

ostream &operator<<(ostream &out, const RewriteHandler &handler) {
    return out << "valid elements: " << handler.valid_elements << " (" << handler.processed_elements << "), "
               << "valid tags: " << handler.valid_tags << " (" << handler.total_tags << ")";
}


class GeoTiff {
    GDALDataset* dataSet;
    OGRCoordinateTransformation* transformation;
    double transform[6] = {};
    int rasterHasNoData = 0;
    double rasterNoDataValue = 0.0;

    public:
        explicit GeoTiff(const char* filename) {
            dataSet = (GDALDataset *) GDALOpenShared(filename, GA_ReadOnly);
            const auto reference = getSpatialReference(dataSet->GetProjectionRef());
            transformation = OGRCreateCoordinateTransformation(&WGS84, &reference);
            dataSet->GetGeoTransform(transform);
            rasterNoDataValue = dataSet->GetRasterBand(1)->GetNoDataValue(&rasterHasNoData);
        }

    static OGRSpatialReference getSpatialReference(const char * crs) {
        OGRSpatialReference reference;
        reference.importFromWkt(crs);
        reference.SetAxisMappingStrategy(OAMS_TRADITIONAL_GIS_ORDER);
        return reference;
    }

    double elevation(double lng, double lat) {
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
                    (rasterHasNoData && pixel[0] <= rasterNoDataValue)) {
                return NO_DATA_VALUE;
            }
            return pixel[0];
        }
};

int main(int argc, char **argv) {
    GDALAllRegister();

    char *filename;
    bool doMemoryCheck = false;
    bool stopAfterMemoryCheck = false;
    bool addElevation = true;
    bool overrideValues = true;

    for (char **arg = argv; *arg; ++arg) {
        if (strcmp(*arg, "-m") == 0) {
            doMemoryCheck = true;
        } else if (strcmp(*arg, "-c") == 0) {
            doMemoryCheck = true;
            stopAfterMemoryCheck = true;
        } else if (strcmp(*arg, "-e") == 0) {
            addElevation = false;
        } else if (strcmp(*arg, "-o") == 0) {
            overrideValues = false;
        } else {
            filename = *arg;
        }
    }
    if (!file_exists(filename)) {
        cerr << "Usage: " << argv[0] << "[OPTIONS] [OSM file]" << endl;
        cerr << "Options:\t-m\tperform memory requirement check" << endl;
        cerr << "\t\t-c\tonly perform memory requirement check" << endl;
        cerr << "\t\t-e\tskip elevation data merge" << endl;
        cerr << "\t\t-o\tkeep original elevation tags where present" << endl;
        return 1;
    }

    string remove_tag_regex_str = "REMOVE NO TAGS";
    bool debug_output = false;
    bool debug_no_filter = false;
    bool debug_no_tag_filter = false;
    llu nodes_max_id;
    llu ways_max_id;
    llu rels_max_id;
    int cache_size = -1;
    try {
        libconfig::Config cfg;
        cfg.readFile("osm-transform.cfg");
        const libconfig::Setting &root = cfg.getRoot();
        root.lookupValue("remove_tag", remove_tag_regex_str);
        root.lookupValue("nodes_max_id", nodes_max_id);
        root.lookupValue("ways_max_id", ways_max_id);
        root.lookupValue("rels_max_id", rels_max_id);
        root.lookupValue("debug_output", debug_output);
        root.lookupValue("debug_no_filter", debug_no_filter);
        root.lookupValue("debug_no_tag_filter", debug_no_tag_filter);
        root.lookupValue("cache_size", cache_size);
        if (debug_no_filter) {
            cout << "DEBUG MODE: Filtering disabled" << endl << endl;
        }
        if (debug_no_tag_filter) {
            cout << "DEBUG MODE: Tag filtering disabled" << endl << endl;
        }
    } catch (const libconfig::FileIOException &fioex) {
        std::cerr << "I/O error while reading config file." << std::endl;
        return 2;
    } catch (const libconfig::ParseException &pex) {
        std::cerr << "Parse error at " << pex.getFile() << ":" << pex.getLine()
                  << " - " << pex.getError() << std::endl;
        return 2;
    } catch (const libconfig::SettingNotFoundException &nfex) {
        cerr << "Missing setting in configuration file: " << nfex.what() << endl;
        return 2;
    }

    ofstream logFile;
    logFile.open("osm-transform.log");
    try {
        if (doMemoryCheck) {
            cout << "Calculating required memory..." << endl;
            osmium::io::Reader check_reader{filename};
            llu insize = check_reader.file_size();
            osmium::ProgressBar check_progress{insize, osmium::isatty(2)};
            MaxIDHandler maxIDHandler;
            while (osmium::memory::Buffer input_buffer = check_reader.read()) {
                osmium::apply(input_buffer, maxIDHandler);
                check_progress.update(check_reader.offset());
            }
            check_progress.done();
            check_progress.remove();
            check_reader.close();
            nodes_max_id = maxIDHandler.node_max_id;
            ways_max_id = maxIDHandler.way_max_id;
            rels_max_id = maxIDHandler.relation_max_id;
            cout << "Max IDs: Node " << maxIDHandler.node_max_id << " Way " << maxIDHandler.way_max_id << " Relation "
                 << maxIDHandler.relation_max_id << endl;
            if (stopAfterMemoryCheck) {
                return 0;
            }
        } else {
            cout << "Max IDs from config: Node " << nodes_max_id << " Way " << ways_max_id << " Relation "
                 << rels_max_id << endl;
        }

        boost::regex remove_tag_regex(remove_tag_regex_str, boost::regex::icase);
        printf("Allocating memory: %.2f Mb nodes, %.2f Mb ways, %.2f Mb relations\n\n",
               nodes_max_id / (1024 * 1024 * 8.0), ways_max_id / (1024 * 1024 * 8.0),
               rels_max_id / (1024 * 1024 * 8.0));
        vi valid_nodes((nodes_max_id / BITWIDTH_INT) + 1, 0);
        vi valid_ways((ways_max_id / BITWIDTH_INT) + 1, 0);
        vi valid_relations((rels_max_id / BITWIDTH_INT) + 1, 0);

        cout << "Processing first pass: validate ways & relations..." << endl;
        auto start = chrono::steady_clock::now();
        osmium::io::Reader first_pass_reader{filename};
        llu insize = first_pass_reader.file_size();
        osmium::ProgressBar progress{insize, osmium::isatty(2)};
        FirstPassHandler first_pass;
        first_pass.init(&remove_tag_regex, &valid_nodes, &valid_ways, &valid_relations, debug_no_filter, nodes_max_id,
                        ways_max_id, rels_max_id);
        while (osmium::memory::Buffer input_buffer = first_pass_reader.read()) {
            osmium::apply(input_buffer, first_pass);
            progress.update(first_pass_reader.offset());
        }
        progress.done();
        progress.remove();
        first_pass_reader.close();
        cout << first_pass << endl;
        auto end = chrono::steady_clock::now();
        printf("Processed in %.3f s\n\n", chrono::duration_cast<chrono::milliseconds>(end - start).count() / 1000.0);

        string output = remove_extension(basename(filename)) + ".ors.pbf";
        llu total_elements = first_pass.node_count + first_pass.way_count + first_pass.relation_count;
        llu processed_elements = 0;
        llu processed_nanos = 0;

        start = chrono::steady_clock::now();
        cout << "Processing second pass: rebuild data..." << endl;
        osmium::io::Reader second_reader{filename};

        // keep existing headers incluing osm data dates
        osmium::io::Header header_in = second_reader.header();
        osmium::io::Header header = header_in;
        header.set("generator", "osm-transform v0.1.0");

        osmium::io::Writer writer{output, header, osmium::io::overwrite::allow};
        RewriteHandler handler(nodes_max_id + 1000000000);
        handler.init(cache_size, &remove_tag_regex, first_pass.valid_nodes, first_pass.valid_ways,
                     first_pass.valid_relations, &logFile, debug_no_filter, debug_no_tag_filter);
        handler.addElevation = addElevation;
        handler.overrideValues = overrideValues;

        string new_node_output = remove_extension(basename(filename)) + ".ors.new_nodes.pbf";
        osmium::io::Writer new_node_writer{new_node_output, header, osmium::io::overwrite::allow};

        while (osmium::memory::Buffer input_buffer = second_reader.read()) {
            auto step_start = chrono::steady_clock::now();

            int bytes_per_cycle = input_buffer.committed();
            osmium::memory::Buffer output_buffer{input_buffer.committed()};
            handler.set_buffer(&output_buffer);

            osmium::memory::Buffer new_node_output_buffer{input_buffer.committed()};
            handler.set_new_node_buffer(&new_node_output_buffer);

            osmium::apply(input_buffer, handler);
            writer(std::move(output_buffer));

            new_node_writer(std::move(new_node_output_buffer));

            auto step_end = chrono::steady_clock::now();
            processed_elements += handler.processed_elements;
            processed_nanos += chrono::duration_cast<chrono::nanoseconds>(step_end - step_start).count();
            printf("\rProgress: %llu / %llu (%3.2f %%)", processed_elements, total_elements,
                   ((float) processed_elements / static_cast<float>(total_elements)) * 100.0);
            if (debug_output) {
                printf(" - Average element process time: %.3f ms - bytes / cycle: %d, %llu elements / cycle",
                       static_cast<float>(processed_nanos) / processed_elements / 1000.0, bytes_per_cycle,
                       handler.processed_elements);
            }
            fflush(stdout);
        }
        second_reader.close();
        writer.close();

        new_node_writer.close();

        end = chrono::steady_clock::now();
        printf("\nProcessed in %.3f s\n", chrono::duration_cast<chrono::milliseconds>(end - start).count() / 1000.0);

        llu outsize = filesize(output);
        llu reduction = insize - outsize;
        printf("\nOriginal: %20llu b\nReduced: %21llu b\nReduction: %19llu b (= %3.2f %%)\n", insize, outsize,
               reduction, (float) reduction / static_cast<float>(insize) * 100);
        if (addElevation) {
            printf("All Nodes: %19llu Nodes\n",
                   countBits(*first_pass.valid_nodes));
            printf("SRTM Elevation: %14.2f %% (%lld)\n",
                   ((double) handler.nodes_with_elevation_srtm_precision) /
                   (double) countBits(*first_pass.valid_nodes) * 100, handler.nodes_with_elevation_srtm_precision);
            printf("GMTED Elevation: %13.2f %% (%lld)\n",
                   ((double) handler.nodes_with_elevation_gmted_precision) /
                   (double) countBits(*first_pass.valid_nodes) * 100, handler.nodes_with_elevation_gmted_precision);
            printf("Failed Elevation: %12.2f %% (%lld)\n",
                   ((double) handler.nodes_with_elevation_not_found /
                    (double) countBits(*first_pass.valid_nodes)) * 100,
                   handler.nodes_with_elevation_not_found);
            if (!overrideValues)
                printf("%30.2f %% already present (%lld)\n",
                       ((float) handler.nodes_with_elevation / static_cast<float>(countBits(*first_pass.valid_nodes))) *
                       100.0,
                       handler.nodes_with_elevation);
        }
        cout << endl;
    } catch (const exception &e) {
        logFile.close();
        cerr << e.what() << '\n';
        return (3);
    }
    logFile.close();
    return 0;
}
