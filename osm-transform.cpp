#include <string>
#include <fstream>
#include <iostream>
#include <chrono>
#include <limits>

#include <boost/regex.hpp>
#include <boost/filesystem.hpp>

#include <boost/geometry.hpp>
#include <boost/geometry/geometries/point.hpp>
#include <boost/geometry/geometries/box.hpp>
#include <boost/geometry/index/rtree.hpp>
#include <libgen.h>
#include <libconfig.h++>

#include "gdal.h"
#include "gdal_priv.h"
#include "gdal_utils.h"
#include "cpl_conv.h"
#include "node-locations.hpp"

#include <osmium/io/any_input.hpp>
#include <osmium/io/any_output.hpp>
#include <osmium/util/file.hpp>
#include <osmium/util/progress_bar.hpp>

#include <vector>
#include <algorithm>

#include "FirstPassHandler.h"
#include "GeoTiff.h"
#include "MaxIdHandler.h"
#include "RewriteHandler.h"

#include <boost/foreach.hpp>

#include "utils.h"

using namespace std;


ostream &operator<<(ostream &out, const FirstPassHandler &handler) {
    return out << "valid nodes: " << countBits(*handler.valid_nodes) << " (" << handler.node_count << "), "
           << "valid ways: " << countBits(*handler.valid_ways) << " (" << handler.way_count << "), "
           << "valid relations: " << countBits(*handler.valid_relations) << " (" << handler.relation_count << ")";
}

ostream &operator<<(ostream &out, const RewriteHandler &handler) {
    return out << "valid elements: " << handler.valid_elements << " (" << handler.processed_elements << "), "
           << "valid tags: " << handler.valid_tags << " (" << handler.total_tags << ")";
}


int main(int argc, char **argv) {
    // int comment(int argc, char **argv) {
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
                   (static_cast<float>(processed_elements) / static_cast<float>(total_elements)) * 100.0);
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
               reduction, static_cast<float>(reduction) / static_cast<float>(insize) * 100);
        if (addElevation) {
            printf("All Nodes: %19llu Nodes\n",
                   countBits(*first_pass.valid_nodes));
            printf("SRTM Elevation: %14.2f %% (%lld)\n",
                   static_cast<double>(handler.nodes_with_elevation_srtm_precision) /
                   static_cast<double>(countBits(*first_pass.valid_nodes)) * 100, handler.nodes_with_elevation_srtm_precision);
            printf("GMTED Elevation: %13.2f %% (%lld)\n",
                   static_cast<double>(handler.nodes_with_elevation_gmted_precision) /
                   static_cast<double>(countBits(*first_pass.valid_nodes)) * 100, handler.nodes_with_elevation_gmted_precision);
            printf("Failed Elevation: %12.2f %% (%lld)\n",
                   (static_cast<double>(handler.nodes_with_elevation_not_found) /
                    static_cast<double>(countBits(*first_pass.valid_nodes))) * 100,
                   handler.nodes_with_elevation_not_found);
            if (!overrideValues)
                printf("%30.2f %% already present (%lld)\n",
                       (static_cast<float>(handler.nodes_with_elevation) / static_cast<float>(countBits(*first_pass.valid_nodes))) *
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


// int main(int argc, char *argv[]) {
int quickTest(int argc, char *argv[]) {
    GDALAllRegister();

    bgi::rtree<rTreeEntry, bgi::quadratic<16>> rtree;

    const std::string path("tiffs");

    auto maxStepWidth = generate_geo_tiff_index(rtree, path);
    // find values intersecting some area defined by a box
    std::cout << "MaxStepWidth: " << maxStepWidth << endl;
    // constexpr box query_box(point(8.852324,49.748482), point(8.852334,49.748492));
    std::vector<rTreeEntry> result_s;
    // auto location = point(15.85232,50.74848);
    // auto location = point(15.85232,50.74848);
    auto location = point(8.85232, 49.74848);
    rtree.query(bgi::contains(location), std::back_inserter(result_s));

    std::cout << "Query output:" << std::endl;

    std::sort(result_s.begin(), result_s.end(), sortRTreeEntryByPrio);

    BOOST_FOREACH(rTreeEntry const& v, result_s) {
        std::cout << bg::wkt<box>(v.first) << " - " << v.second.prio << " - " << v.second.fileName << std::endl;
        GeoTiff geo_tiff(v.second.fileName.c_str());
        std::cout << "Elevation in point " << bg::wkt<point>(location) << ": " << geo_tiff.elevation(location.get<0>(), location.get<1>()) << std::endl;
    }

    return 0;
}
