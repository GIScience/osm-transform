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
    std::uint32_t cache_limit;
    std::float_t interpolate_threshold;
    std::string index_type;
    std::string area_mapping;
    std::uint16_t area_mapping_id_col;
    std::uint16_t area_mapping_geo_col;
    std::string area_mapping_geo_type;
    bool area_mapping_has_header;
    std::string area_mapping_processed_file_prefix;
    bool download_srtm = false;
    bool download_gmted = false;

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
                ("srtm", "fetch SRTM tiles and exit")
                ("gmted", "fetch GMTED tiles and exit")
                ("interpolate,i", "interpolate intermediate nodes")
                ("remove_tag,T", po::value<std::string>(&remove_tag_regex_str)->default_value("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia"), "regex to match removable tags")
                ("geo_tiff_folders,F", po::value<std::vector<std::string>>(&geo_tiff_folders)->multitoken()->default_value(std::vector<std::string>{"tiffs", "srtmdata", "gmteddata"}, "tiffs, srtmdata, gmteddata"), "paths to geotiff folders")
                ("cache_limit,S", po::value<std::uint32_t>(&cache_limit)->default_value(1073741824), "maximum memory used to store tiles in cache")
                ("threshold,t", po::value<std::float_t>(&interpolate_threshold)->default_value(0.5), "only used in combination with interpolation, threshold for elevation")
                ("area_mapping,a", po::value<std::string>(&area_mapping), "path to area mapping file to use")
                ("area_mapping_id_col", po::value<std::uint16_t>(&area_mapping_id_col)->default_value(0), "column number (zero-based) in area mapping file of area id")
                ("area_mapping_geo_col", po::value<std::uint16_t>(&area_mapping_geo_col)->default_value(1), "column number (zero-based) in area mapping file of area geometry")
                ("area_mapping_geo_type", po::value<std::string>(&area_mapping_geo_type)->default_value("wkt"), "type of geometry string in area mapping file (possible values: 'wkt' (default), 'geojson')")
                ("area_mapping_has_header", po::value<bool>(&area_mapping_has_header)->default_value(true), "area mapping file has header row")
                ("area_mapping_processed_file_prefix", po::value<std::string>(&area_mapping_processed_file_prefix)->default_value("mapping_"), "file prefix for processed mapping files")
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

        debug_mode = vm.contains("debug_mode");
        if (debug_mode) {
            std::cout << "DEBUG MODE" << "\n";
        }

        download_srtm = vm.contains("srtm");
        download_gmted = vm.contains("gmted");
        if (download_srtm || download_gmted) {
            return;
        }

        if (!vm.contains("osm_pbf")) {
            std::cerr << "no file name" << std::endl;
            exit(1);
        }

        filename = vm["osm_pbf"].as<std::vector<std::string>>()[0];

        if (!std::filesystem::exists(filename)) {
            std::cerr << "osm_pbf does not exist " << filename << std::endl;
            exit(1);
        }

        if (vm.contains("interpolate")) {
            interpolate = true;
        }

        if (vm.contains("skip_elevation")) {
            add_elevation = false;
        }
    }
};


#endif//OSM_TRANSFORM_CONFIG_H
