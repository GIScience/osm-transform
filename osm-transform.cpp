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
#include <boost/program_options.hpp>

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


struct Config {
    std::string filename;
    std::string remove_tag_regex_str;

    bool doMemoryCheck = false;
    bool stopAfterMemoryCheck = false;
    bool addElevation = true;
    bool overrideValues = true;

    bool debug_output = false;
    bool debug_no_filter = false;
    bool debug_no_tag_filter = false;

    llu nodes_max_id;
    llu ways_max_id;
    llu rels_max_id;

    int cache_size = -1;

    auto cmd(int argc, char **argv) {

        namespace po = boost::program_options;
        string config_file_path;

        // Declare a group of options that will be
        // allowed only on command line
        po::options_description generic("Generic options");
        generic.add_options()
                ("version,v", "print version string") //
                ("help", "produce help message") //
                (",m", "perform memory requirement check") //
                (",c", "only perform memory requirement check") //
                (",e", "skip elevation data merge") //
                (",o", "keep original elevation tags where present") //
                ("osm-pbf,p", po::value<vector<string>>(), "Absolute file path to osm pbf file to process.") //
                ("config-file,f", po::value<string>(&config_file_path), "Absolute file path to config file to use");



        po::options_description config("Configuration");
        vector<string> geo_tiff_folder;
        config.add_options()
                ("remove_tag,T", po::value<string>(&remove_tag_regex_str)->default_value("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia"), "Regex to match removable tags")
                ("geo_tiff_folders,F", po::value<vector<string>>(&geo_tiff_folder)->composing(), "Absolute paths to GeoTiff folders. Default: srtmdata")
                ("cache_tile_size,S", po::value<int>(&cache_size)->default_value(10), "Maximum amount of tiles stored in cache")
                ("nodes_max_id,N", po::value<llu>(&nodes_max_id)->default_value(11000000000L), "Max Node Id")
                ("ways_max_id,W", po::value<llu>(&ways_max_id)->default_value(1200000000L), "Max Ways Id")
                ("rels_max_id,R", po::value<llu>(&rels_max_id)->default_value(20000000L), "Max Rels Id")
                ("debug_output", "debug_output")
                ("debug_no_filter", "debug_no_filter")
                ("debug_no_tag_filter", "debug_no_tag_filter");



        // Hidden options, will be allowed both on command line and
        // in config file, but will not be shown to the user.
        po::options_description hidden("Hidden options");

        po::options_description cmdline_options;
        cmdline_options.add(generic).add(config).add(hidden);

        po::options_description config_file_options;
        config_file_options.add(config).add(hidden);

        po::options_description visible("Allowed options");
        visible.add(generic).add(config);

        po::positional_options_description p;
        p.add("osm-pbf", 1);
        po::variables_map vm;

        try {
            po::store(po::command_line_parser(argc, argv).options(cmdline_options).positional(p).run(), vm);
            po::notify(vm);
        } catch (boost::program_options::unknown_option e) {
            std::cerr << e.what() << endl;
            cout << visible << "\n";
            exit(1);
        }
        try {
            if (file_exists(config_file_path)) {
                po::store(po::parse_config_file(config_file_path.c_str(), config_file_options, false),vm);
                po::notify(vm);
            }
        } catch (boost::program_options::unknown_option e) {
            std::cerr << e.what() << "  in config file " << config_file_path << endl;
            cout << config << "\n";
            exit(1);
        }

        if (!vm.contains("osm-pbf")) {
            std::cerr << "no file name" << endl;
            exit(1);
        }

        filename = vm["osm-pbf"].as<std::vector<std::string>>()[0];

        if (!file_exists(filename)) {
            std::cerr << "osm-pbf does not exist " << filename << std::endl;
            exit(1);
        }

        if (vm.contains("help")) {
            cout << visible << "\n";
            exit(1);
        }

        if (vm.contains("m")) {
            doMemoryCheck = true;
        }

        if (vm.contains("c")) {
            doMemoryCheck = true;
            stopAfterMemoryCheck = true;
        }

        if (vm.contains("e")) {
            addElevation = false;
        }

        if (vm.contains("o")) {
            overrideValues = false;
        }

        debug_output = vm.contains("debug_output");
        debug_no_filter = vm.contains("debug_no_filter");
        debug_no_tag_filter = vm.contains("debug_no_tag_filter");

        if (debug_no_filter) {
            cout << "DEBUG MODE: Filtering disabled" << endl << endl;
        }
        if (debug_no_tag_filter) {
            cout << "DEBUG MODE: Tag filtering disabled" << endl << endl;
        }
    }
};

int main(int argc, char **argv) {
    Config config;
    config.cmd(argc, argv);

    GDALAllRegister();


    ofstream logFile;
    logFile.open("osm-transform.log");
    try {
        if (config.doMemoryCheck) {
            cout << "Calculating required memory..." << endl;
            osmium::io::Reader check_reader{config.filename};
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
            config.nodes_max_id = maxIDHandler.node_max_id;
            config.ways_max_id = maxIDHandler.way_max_id;
            config.rels_max_id = maxIDHandler.relation_max_id;
            cout << "Max IDs: Node " << maxIDHandler.node_max_id << " Way " << maxIDHandler.way_max_id << " Relation "
                 << maxIDHandler.relation_max_id << endl;
            if (config.stopAfterMemoryCheck) {
                return 0;
            }
        } else {
            cout << "Max IDs from config: Node " << config.nodes_max_id << " Way " << config.ways_max_id << " Relation "
                 << config.rels_max_id << endl;
        }

        boost::regex remove_tag_regex(config.remove_tag_regex_str, boost::regex::icase);
        printf("Allocating memory: %.2f Mb nodes, %.2f Mb ways, %.2f Mb relations\n\n",
               config.nodes_max_id / (1024 * 1024 * 8.0), config.ways_max_id / (1024 * 1024 * 8.0),
               config.rels_max_id / (1024 * 1024 * 8.0));
        vi valid_nodes((config.nodes_max_id / BITWIDTH_INT) + 1, 0);
        vi valid_ways((config.ways_max_id / BITWIDTH_INT) + 1, 0);
        vi valid_relations((config.rels_max_id / BITWIDTH_INT) + 1, 0);

        cout << "Processing first pass: validate ways & relations..." << endl;
        auto start = chrono::steady_clock::now();
        osmium::io::Reader first_pass_reader{config.filename};
        llu insize = first_pass_reader.file_size();
        osmium::ProgressBar progress{insize, osmium::isatty(2)};
        FirstPassHandler first_pass;
        first_pass.init(&remove_tag_regex, &valid_nodes, &valid_ways, &valid_relations, config.debug_no_filter, config.nodes_max_id,
                        config.ways_max_id, config.rels_max_id);
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

        string output = remove_extension(fs::path(config.filename.c_str()).stem()) + ".ors.pbf";
        llu total_elements = first_pass.node_count + first_pass.way_count + first_pass.relation_count;
        llu processed_elements = 0;
        llu processed_nanos = 0;

        start = chrono::steady_clock::now();
        cout << "Processing second pass: rebuild data..." << endl;
        osmium::io::Reader second_reader{config.filename};

        // keep existing headers incluing osm data dates
        osmium::io::Header header_in = second_reader.header();
        osmium::io::Header header = header_in;
        header.set("generator", "osm-transform v0.1.0");

        osmium::io::Writer writer{output, header, osmium::io::overwrite::allow};
        RewriteHandler handler(config.nodes_max_id + 1000000000);
        handler.init(config.cache_size, &remove_tag_regex, first_pass.valid_nodes, first_pass.valid_ways,
                     first_pass.valid_relations, &logFile, config.debug_no_filter, config.debug_no_tag_filter);
        handler.addElevation = config.addElevation;
        handler.overrideValues = config.overrideValues;;


        string new_node_output = remove_extension(fs::path(config.filename.c_str()).stem()) + ".ors.new_nodes.pbf";
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
            if (config.debug_output) {
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
        if (config.addElevation) {
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
            if (!config.overrideValues)
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
