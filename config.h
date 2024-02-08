#ifndef OSM_TRANSFORM_CONFIG_H
#define OSM_TRANSFORM_CONFIG_H

#include <iostream>
#include <filesystem>
#include <boost/program_options.hpp>

struct Config {
    std::string filename;
    std::string remove_tag_regex_str;

    bool add_elevation = true;
    bool override_values = true;
    bool interpolate = false;

    bool debug_output = false;

    int cache_limit = -1;

    auto cmd(int argc, char **argv) {

        namespace po = boost::program_options;
        std::string config_file_path;

        // Declare a group of options that will be
        // allowed only on command line
        po::options_description generic("Generic options");
        generic.add_options()
                ("version,v", "print version string") //
                ("help", "produce help message") //
                ("skip,e", "skip elevation data merge") //
                ("overwrite,o", "keep original elevation tags where present") //
                ("interpolate,i", "interpolate intermediate nodes")
                ("osm-pbf,p", po::value<std::vector<std::string>>(), "Absolute file path to osm pbf file to process.") //
                ("config-file,f", po::value<std::string>(&config_file_path), "Absolute file path to config file to use");

        po::options_description config("Configuration");
        std::vector<std::string> geo_tiff_folder;
        config.add_options()
                ("remove_tag,T", po::value<std::string>(&remove_tag_regex_str)->default_value("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia"), "Regex to match removable tags")
                ("geo_tiff_folders,F", po::value<std::vector<std::string>>(&geo_tiff_folder)->composing(), "Absolute paths to Geotiff folders. Default: srtmdata")
                ("cache_limit,S", po::value<int>(&cache_limit)->default_value(1073741824), "Maximum memory used to store tiles in cache")
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
            std::cerr << e.what() << std::endl;
            std::cout << visible << "\n";
            exit(1);
        }
        try {
            if (std::filesystem::exists(config_file_path)) {
                po::store(po::parse_config_file(config_file_path.c_str(), config_file_options, false),vm);
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

        if (vm.contains("overwrite")) {
            override_values = false;
        }

        debug_output = vm.contains("debug_output");
    }
};


#endif//OSM_TRANSFORM_CONFIG_H
