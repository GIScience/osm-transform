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

#include "cpl_conv.h"

#include <osmium/io/any_input.hpp>
#include <osmium/io/any_output.hpp>
#include <osmium/util/file.hpp>
#include <osmium/util/progress_bar.hpp>
#include <osmium/index/index.hpp>
#include <osmium/index/map/flex_mem.hpp>
#include <osmium/index/map/all.hpp>
#include <osmium/util/memory.hpp>

#include <osmium/index/id_set.hpp>
#include <osmium/index/nwr_array.hpp>

#include <osmium/handler.hpp>
#include <osmium/handler/node_locations_for_ways.hpp>
#include <osmium/util/progress_bar.hpp>
#include <osmium/visitor.hpp>
#include <osmium/osm/node_ref.hpp>


#include <vector>
#include <algorithm>

#include "FirstPassHandler.h"
#include "RewriteHandler.h"

#include "utils.h"

using namespace std;


ostream &operator<<(ostream &out, const FirstPassHandler &handler) {
    return out << "valid nodes: " << handler.m_valid_ids->nodes().size() << " (" << handler.node_count << "), "
           << "valid ways: " << handler.m_valid_ids->ways().size() << " (" << handler.way_count << "), "
           << "valid relations: " << handler.m_valid_ids->relations().size() << " (" << handler.relation_count << ")";
}

ostream &operator<<(ostream &out, const RewriteHandler &handler) {
    return out << "valid elements: " << handler.valid_elements << " (" << handler.processed_elements << "), "
           << "valid tags: " << handler.valid_tags << " (" << handler.total_tags << ")";
}


struct Config {
    std::string filename;
    std::string remove_tag_regex_str;

    bool addElevation = true;
    bool overrideValues = true;

    bool debug_output = false;

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
                ("debug_output", "debug_output");


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
        } catch (boost::program_options::unknown_option &e) {
            std::cerr << e.what() << endl;
            cout << visible << "\n";
            exit(1);
        }
        try {
            if (file_exists(config_file_path)) {
                po::store(po::parse_config_file(config_file_path.c_str(), config_file_options, false),vm);
                po::notify(vm);
            }
        } catch (boost::program_options::unknown_option &e) {
            std::cerr << e.what() << "  in config file " << config_file_path << endl;
            cout << config << "\n";
            exit(1);
        }

        if (vm.contains("help")) {
            cout << visible << "\n";
            exit(1);
        }

        if (vm.contains("version")) {
            cout << PROJECT_NAME << " " << PROJECT_VERSION << "\n";
            exit(0);
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

        if (vm.contains("e")) {
            addElevation = false;
        }

        if (vm.contains("o")) {
            overrideValues = false;
        }

        debug_output = vm.contains("debug_output");
    }
};


auto show_memory_used() {
    const osmium::MemoryUsage mem;
    if (mem.current() > 0) {
        std::cout << "Peak memory used: " << mem.peak() << " MBytes\n";
    }
}


int main(int argc, char **argv) {
    Config config;
    config.cmd(argc, argv);

    ofstream logFile;
    logFile.open("osm-transform.log");
    try {

        boost::regex remove_tag_regex(config.remove_tag_regex_str, boost::regex::icase);

        osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> valid_ids;

        cout << "Processing first pass: validate ways & relations..." << endl;
        auto start = chrono::steady_clock::now();
        osmium::io::Reader first_pass_reader{config.filename};
        llu insize = first_pass_reader.file_size();
        osmium::ProgressBar progress{insize, osmium::isatty(2)};
        FirstPassHandler first_pass(&remove_tag_regex, &valid_ids);
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

        // second pass


        using index_type = osmium::index::map::Map<osmium::unsigned_object_id_type, osmium::Location>;
        const auto& map_factory = osmium::index::MapFactory<osmium::unsigned_object_id_type, osmium::Location>::instance();
        auto location_index = map_factory.create_map("flex_mem");

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
        RewriteHandler handler(1000000000, location_index, config.cache_size, &remove_tag_regex, &valid_ids, &logFile);
        handler.addElevation = config.addElevation;
        handler.overrideValues = config.overrideValues;;


        string new_node_output = remove_extension(fs::path(config.filename.c_str()).stem()) + ".ors.new_nodes.pbf";
        osmium::io::Writer new_node_writer{new_node_output, header, osmium::io::overwrite::allow};

        while (osmium::memory::Buffer input_buffer = second_reader.read()) {
            auto step_start = chrono::steady_clock::now();

            auto bytes_per_cycle = input_buffer.committed();
            osmium::memory::Buffer output_buffer{bytes_per_cycle};
            osmium::memory::Buffer new_node_output_buffer{input_buffer.committed()};
            handler.set_buffers(&output_buffer, &new_node_output_buffer);

            osmium::apply(input_buffer, handler);
            writer(std::move(output_buffer));
            new_node_writer(std::move(new_node_output_buffer));

            auto step_end = chrono::steady_clock::now();
            processed_elements += handler.processed_elements;
            processed_nanos += chrono::duration_cast<chrono::nanoseconds>(step_end - step_start).count();
            printf("\rProgress: %llu / %llu (%3.2f %%)", processed_elements, total_elements,
                   (static_cast<float>(processed_elements) / static_cast<float>(total_elements)) * 100.0);
            if (config.debug_output) {
                printf(" - Average element process time: %.3f ms - bytes / cycle: %ld, %llu elements / cycle",
                       static_cast<float>(processed_nanos) / processed_elements / 1000.0, bytes_per_cycle,
                       handler.processed_elements);
            }
            fflush(stdout);
        }
        second_reader.close();
        writer.close();

        new_node_writer.close();

        const auto mem = location_index->used_memory() / (1024UL );
        std::cout << "\nAbout " << mem << " KBytes used for node location index (in main memory or on disk).\n";

        end = chrono::steady_clock::now();
        printf("\nProcessed in %.3f s\n", chrono::duration_cast<chrono::milliseconds>(end - start).count() / 1000.0);

        llu outsize = filesize(output);
        llu reduction = insize - outsize;
        printf("\nOriginal: %20llu b\nReduced: %21llu b\nReduction: %19llu b (= %3.2f %%)\n", insize, outsize,
               reduction, static_cast<float>(reduction) / static_cast<float>(insize) * 100);
        if (config.addElevation) {
            auto valid_nodes = valid_ids.nodes().size();
            printf("All Nodes: %19llu Nodes\n",
                   valid_nodes);
            printf("Custom Elevation: %14.2f %% (%lld)\n",
                   static_cast<double>(handler.nodes_with_elevation_high_precision) /
                   static_cast<double>(valid_nodes) * 100, handler.nodes_with_elevation_high_precision);
            printf("SRTM Elevation: %14.2f %% (%lld)\n",
                   static_cast<double>(handler.nodes_with_elevation_srtm_precision) /
                   static_cast<double>(valid_nodes) * 100, handler.nodes_with_elevation_srtm_precision);
            printf("GMTED Elevation: %13.2f %% (%lld)\n",
                   static_cast<double>(handler.nodes_with_elevation_gmted_precision) /
                   static_cast<double>(valid_nodes) * 100, handler.nodes_with_elevation_gmted_precision);
            printf("Failed Elevation: %12.2f %% (%lld)\n",
                   static_cast<double>(handler.nodes_with_elevation_not_found) /
                   static_cast<double>(valid_nodes) * 100,
                   handler.nodes_with_elevation_not_found);
            if (!config.overrideValues)
                printf("%30.2f %% already present (%lld)\n",
                       (static_cast<float>(handler.nodes_with_elevation) / static_cast<float>(valid_nodes)) *
                       100.0,
                       handler.nodes_with_elevation);
        }
        cout << endl;

        show_memory_used();

    } catch (const exception &e) {
        logFile.close();
        cerr << e.what() << '\n';
        return (3);
    }
    logFile.close();
    return 0;
}
