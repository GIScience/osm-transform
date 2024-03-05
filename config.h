#ifndef OSM_TRANSFORM_CONFIG_H
#define OSM_TRANSFORM_CONFIG_H

#include <iostream>
#include <filesystem>
#include <boost/program_options.hpp>

struct Config {
    std::string filename;
    std::string remove_tag_regex_str;
    std::vector<std::string> geo_tiff_folders;
    bool add_elevation = true;
    bool interpolate = false;
    bool debug_mode = false;
    int cache_limit;
    double interpolate_threshold;
    std::string index_type;

    auto cmd(int argc, char **argv) {

        namespace po = boost::program_options;
        std::string config_file_path;

        // Declare a group of options that will be
        // allowed only on command line
        po::options_description generic("Generic options");
        generic.add_options()
                ("version,v", "print version string")
                ("help,h", "produce help message");

        po::options_description config("Configuration");
        config.add_options()
                ("osm_pbf,p", po::value<std::vector<std::string>>(), "path to osm pbf file to process")
                ("skip_elevation,e", "skip elevation data merge")
                ("interpolate,i", "interpolate intermediate nodes")
                ("remove_tag,T", po::value<std::string>(&remove_tag_regex_str)->default_value("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia"), "regex to match removable tags")
                ("geo_tiff_folders,F", po::value<std::vector<std::string>>(&geo_tiff_folders)->multitoken()->default_value(std::vector<std::string>{"tiffs", "srtmdata", "gmteddata"}, "tiffs, srtmdata, gmteddata"), "paths to geotiff folders")
                ("cache_limit,S", po::value<int>(&cache_limit)->default_value(1073741824), "maximum memory used to store tiles in cache")
                ("threshold,t", po::value<double>(&interpolate_threshold)->default_value(0.5), "only used in combination with interpolation, threshold for elevation")
                ("config_file,f", po::value<std::string>(&config_file_path), "path to config file to use")
                ("index_type", po::value<std::string>(&index_type)->default_value("flex_mem"), "index type for locations, needed for interpolate. see https://docs.osmcode.org/osmium/latest/osmium-index-types.html")
                ("debug_mode,d", "debug_mode");

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
            std::cerr << e.what() << std::endl;
            std::cout << visible << "\n";
            exit(1);
        }
        try {
            if (std::filesystem::exists(config_file_path)) {
                po::store(po::parse_config_file(config_file_path.c_str(), config_file_options, false), vm);
                po::notify(vm);
            }
        } catch (boost::program_options::unknown_option &e) {
            std::cerr << e.what() << "  in config file " << config_file_path << std::endl;
            std::cout << config << "\n";
            exit(1);
        }

        if (vm.contains("help")) {
            std::cout << visible << "\n";
            exit(1);
        }

        if (vm.contains("version")) {
            std::cout << PROJECT_NAME << " " << PROJECT_VERSION << "\n";
            exit(0);
        }

        if (!vm.contains("osm-pbf")) {
            std::cerr << "no file name" << std::endl;
            exit(1);
        }

        filename = vm["osm-pbf"].as<std::vector<std::string>>()[0];

        if (!std::filesystem::exists(filename)) {
            std::cerr << "osm-pbf does not exist " << filename << std::endl;
            exit(1);
        }

        if (vm.contains("interpolate")) {
            interpolate = true;
        }

        if (vm.contains("skip")) {
            add_elevation = false;
        }

        debug_mode = vm.contains("debug_mode");
        if (debug_mode) {
            std::cout << "DEBUG MODE" << "\n";
        }
    }
};


#endif//OSM_TRANSFORM_CONFIG_H
