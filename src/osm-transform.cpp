#include "config.h"
#include "firstpass_handler.h"
#include "rewrite_handler.h"

#include <chrono>
#include <filesystem>
#include <iostream>
#include <string>

#include <boost/regex.hpp>

#include <osmium/io/any_input.hpp>
#include <osmium/io/any_output.hpp>
#include <osmium/util/file.hpp>
#include <osmium/util/progress_bar.hpp>
#include <osmium/util/memory.hpp>
#include <osmium/visitor.hpp>

using namespace std;

auto remove_extension(const string &filename) {
    const size_t lastdot = filename.find_first_of('.');
    if (lastdot == string::npos) return filename;
    return filename.substr(0, lastdot);
}

auto show_memory_used() {
    const osmium::MemoryUsage mem;
    if (mem.current() > 0) {
        std::cout << "Peak memory used: " << mem.peak() << " MBytes\n";
    }
}

void first_pass(Config &config, boost::regex &remove_tag_regex, osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids, osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> &no_elevation);
void second_pass(Config &config, boost::regex &remove_tag_regex, osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids, osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> &no_elevation);

int main(int argc, char **argv) {
    Config config;
    config.cmd(argc, argv);
    try {
        boost::regex remove_tag_regex(config.remove_tag_regex_str, boost::regex::icase);
        osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> valid_ids;
        osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> no_elevation;

        first_pass(config, remove_tag_regex, valid_ids, no_elevation);
        second_pass(config, remove_tag_regex, valid_ids, no_elevation);
        show_memory_used();
    } catch (const exception &e) {
        cerr << e.what() << '\n';
        return (3);
    }
    return 0;
}

void first_pass(Config &config, boost::regex &remove_tag_regex,
                osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids,
                osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> &no_elevation) {
    cout << "Processing first pass: validate ways & relations..." << endl;
    auto start = chrono::steady_clock::now();

    osmium::io::Reader reader{config.filename, osmium::osm_entity_bits::way | osmium::osm_entity_bits::relation,  osmium::io::read_meta::no};
    osmium::ProgressBar progress{reader.file_size(), osmium::isatty(2)};
    FirstPassHandler handler(remove_tag_regex, valid_ids, no_elevation);
    while (osmium::memory::Buffer input_buffer = reader.read()) {
        osmium::apply(input_buffer, handler);
        progress.update(reader.offset());
    }
    progress.done();
    progress.remove();
    reader.close();

    handler.printStats();

    printf("Processed in %.3f s\n\n", chrono::duration_cast<chrono::milliseconds>(chrono::steady_clock::now() - start).count() / 1000.0);
}


void copy(const std::string& input, osmium::io::Writer& writer) {
    osmium::io::Reader reader{input};
    osmium::ProgressBar progress{reader.file_size(), osmium::isatty(2)};
    while (osmium::memory::Buffer buffer = reader.read()) {
        writer(std::move(buffer));
        progress.update(reader.offset());
    }
    reader.close();
}

void second_pass(Config &config, boost::regex &remove_tag_regex,
                 osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids,
                 osmium::nwr_array<osmium::index::IdSetSmall<osmium::unsigned_object_id_type>> &no_elevation) {
    LocationElevationService location_elevation_service(config.cache_limit, config.debug_mode);
    if (config.add_elevation) {
        auto start = chrono::steady_clock::now();
        location_elevation_service.load(config.geo_tiff_folders);
        printf("Processed in %.3f s\n\n", chrono::duration_cast<chrono::milliseconds>(chrono::steady_clock::now() - start).count() / 1000.0);
    }

    LocationAreaService location_area_service(config.debug_mode, config.area_mapping_id_col, config.area_mapping_geo_col, config.area_mapping_geo_type, config.area_mapping_has_header, config.area_mapping_processed_file_prefix);
    if (!config.area_mapping.empty()) {
        auto start = chrono::steady_clock::now();
        location_area_service.load(config.area_mapping);
        printf("Processed in %.3f s\n\n", chrono::duration_cast<chrono::milliseconds>(chrono::steady_clock::now() - start).count() / 1000.0);
    }

    const auto& map_factory = osmium::index::MapFactory<osmium::unsigned_object_id_type, osmium::Location>::instance();
    auto location_index = map_factory.create_map(config.index_type);

    auto output = remove_extension(std::filesystem::path(config.filename.c_str()).stem()) + ".ors.pbf";
    const auto total_elements = valid_ids.nodes().size() + valid_ids.ways().size() + valid_ids.relations().size();
    unsigned long long processed_elements = 0;

    const auto start = chrono::steady_clock::now();
    cout << "Processing second pass: rebuild data..." << endl;
    osmium::io::Reader reader{config.filename, osmium::osm_entity_bits::node | osmium::osm_entity_bits::way | osmium::osm_entity_bits::relation, osmium::io::read_meta::no};

    // keep existing headers including osm data dates
    osmium::io::Header header(reader.header());
    header.set("generator", "osm-transform_ v0.1.0");

    RewriteHandler handler(1000000000, location_index, location_elevation_service, location_area_service, remove_tag_regex, valid_ids, no_elevation, config.interpolate, config.interpolate_threshold);
    handler.add_elevation_ = config.add_elevation;


    if (config.interpolate) {
        auto wr_output = remove_extension(std::filesystem::path(config.filename.c_str()).stem()) + ".ors.wr.pbf";
        osmium::io::Writer wr_writer{wr_output, header, osmium::io::overwrite::allow};
        const auto n_output = remove_extension(std::filesystem::path(config.filename.c_str()).stem()) + ".ors.n.pbf";
        osmium::io::Writer n_writer{n_output, header, osmium::io::overwrite::allow};
        osmium::ProgressBar progress{total_elements, osmium::isatty(2)};
        while (auto input_buffer = reader.read()) {
            osmium::memory::Buffer output_buffer{input_buffer.committed()};
            osmium::memory::Buffer node_output_buffer{input_buffer.committed()};
            handler.set_buffers(&output_buffer, &node_output_buffer);

            osmium::apply(input_buffer, handler);
            wr_writer(std::move(output_buffer));
            n_writer(std::move(node_output_buffer));

            processed_elements += handler.processed_elements_;
            progress.update(processed_elements);
        }
        n_writer.close();
        wr_writer.close();
        progress.done();
        reader.close();


        osmium::io::Writer writer{output, header, osmium::io::overwrite::allow};
        auto total_size = std::filesystem::file_size(n_output) + std::filesystem::file_size(wr_output);
        copy(n_output, writer);
        std::remove(n_output.c_str());
        copy(wr_output, writer);
        std::remove(wr_output.c_str());
        writer.close();
    } else {
        osmium::io::Writer writer{output, header, osmium::io::overwrite::allow};
        osmium::ProgressBar progress{total_elements, osmium::isatty(2)};
        while (auto input_buffer = reader.read()) {
            osmium::memory::Buffer output_buffer{input_buffer.committed()};
            handler.set_buffers(&output_buffer, &output_buffer);

            osmium::apply(input_buffer, handler);
            writer(std::move(output_buffer));

            processed_elements += handler.processed_elements_;
            progress.update(processed_elements);
        }
        writer.close();
        progress.done();
        reader.close();
    }

    if (config.debug_mode)  {
        const auto mem = location_index->used_memory() / (1024UL );
        std::cout << "About " << mem << " KBytes used for node location index (in main memory or on disk).\n";
    }

    const auto end = chrono::steady_clock::now();
    printf("Processed in %.3f s\n", chrono::duration_cast<chrono::milliseconds>(end - start).count() / 1000.0);

    const auto insize = std::filesystem::file_size(config.filename);
    const auto outsize = std::filesystem::file_size(output);
    const auto reduction = insize - outsize;
    printf("\nOriginal: %20ju b\nReduced: %21lu b\nReduction: %19ju b (= %3.2f %%)\n", insize, outsize,
           reduction, static_cast<float>(reduction) / static_cast<float>(insize) * 100);
    if (config.add_elevation) {
        auto valid_nodes = valid_ids.nodes().size();
        printf("All Nodes: %19lu Nodes\n",valid_nodes);
        if (config.interpolate) {
            printf("Added Nodes: %17llu Nodes\n",handler.nodes_added_by_interpolation_);
        }
        printf("Elevation found: %13.2f %% (%llu)\n",
               static_cast<double>(handler.nodes_with_elevation_) /
                       static_cast<double>(valid_nodes) * 100, handler.nodes_with_elevation_);
        printf("Custom Elevation: %12.2f %% (%llu)\n",
               static_cast<double>(location_elevation_service.found_custom_) /
                       static_cast<double>(valid_nodes) * 100, location_elevation_service.found_custom_);
        printf("SRTM Elevation: %14.2f %% (%llu)\n",
               static_cast<double>(location_elevation_service.found_srtm_) /
                       static_cast<double>(valid_nodes) * 100, location_elevation_service.found_srtm_);
        printf("GMTED Elevation: %13.2f %% (%llu)\n",
               static_cast<double>(location_elevation_service.found_gmted_) /
                       static_cast<double>(valid_nodes) * 100, location_elevation_service.found_gmted_);
        printf("Failed Elevation: %12.2f %% (%llu)\n",
               static_cast<double>(handler.nodes_with_elevation_not_found_) /
                       static_cast<double>(valid_nodes) * 100, handler.nodes_with_elevation_not_found_);
        if (valid_nodes > handler.nodes_with_elevation_ + handler.nodes_with_elevation_not_found_) {
            std::cout << "\nNotice: More nodes were referenced in ways & relations than were found in the data. This typically happens\n"
                         "with OSM extracts with nodes omitted for ways & relations extending beyond the extent of the extract.\n";
        }
    }
    cout << endl;
}


