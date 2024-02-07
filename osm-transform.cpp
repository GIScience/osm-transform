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

ostream &operator<<(ostream &out, const FirstPassHandler &handler) {
    return out << "valid nodes: " << handler.valid_ids_.nodes().size() << " (" << handler.node_count_ << "), "
           << "valid ways: " << handler.valid_ids_.ways().size() << " (" << handler.way_count_ << "), "
           << "valid relations: " << handler.valid_ids_.relations().size() << " (" << handler.relation_count_ << ")";
}

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

void first_pass(Config &config, boost::regex &remove_tag_regex, osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids);
void second_pass(Config &config, boost::regex &remove_tag_regex, osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids);

int main(int argc, char **argv) {
    Config config;
    config.cmd(argc, argv);

    try {

        boost::regex remove_tag_regex(config.remove_tag_regex_str, boost::regex::icase);
        osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> valid_ids;

        first_pass(config, remove_tag_regex, valid_ids);
        second_pass(config, remove_tag_regex, valid_ids);

        show_memory_used();

    } catch (const exception &e) {
        cerr << e.what() << '\n';
        return (3);
    }
    return 0;
}

void first_pass(Config &config, boost::regex &remove_tag_regex, osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids) {
    cout << "Processing first pass: validate ways & relations..." << endl;
    auto start = chrono::steady_clock::now();

    osmium::io::Reader reader{config.filename, osmium::osm_entity_bits::way | osmium::osm_entity_bits::relation,  osmium::io::read_meta::no};
    osmium::ProgressBar progress{reader.file_size(), osmium::isatty(2)};
    FirstPassHandler handler(remove_tag_regex, valid_ids);
    while (osmium::memory::Buffer input_buffer = reader.read()) {
        osmium::apply(input_buffer, handler);
        progress.update(reader.offset());
    }
    progress.done();
    progress.remove();
    reader.close();

    cout << handler << endl;

    printf("Processed in %.3f s\n\n", chrono::duration_cast<chrono::milliseconds>(chrono::steady_clock::now() - start).count() / 1000.0);
}

void second_pass(Config &config, boost::regex &remove_tag_regex, osmium::nwr_array<osmium::index::IdSetDense<osmium::unsigned_object_id_type>> &valid_ids) {
    LocationElevationService location_elevation_service(config.cache_size);
    location_elevation_service.load("tiffs");

    const auto& map_factory = osmium::index::MapFactory<osmium::unsigned_object_id_type, osmium::Location>::instance();
    auto location_index = map_factory.create_map("flex_mem");

    string output = remove_extension(std::filesystem::path(config.filename.c_str()).stem()) + ".ors.pbf";
    const auto total_elements = valid_ids.nodes().size() + valid_ids.ways().size() + valid_ids.relations().size();
    unsigned long long processed_elements = 0;

    const auto start = chrono::steady_clock::now();
    cout << "Processing second pass: rebuild data..." << endl;
    osmium::io::Reader reader{config.filename, osmium::osm_entity_bits::node | osmium::osm_entity_bits::way | osmium::osm_entity_bits::relation, osmium::io::read_meta::no};

    // keep existing headers incluing osm data dates
    osmium::io::Header header(reader.header());
    header.set("generator", "osm-transform_ v0.1.0");

    osmium::io::Writer writer{output, header, osmium::io::overwrite::allow};
    RewriteHandler handler(1000000000, location_index, location_elevation_service, remove_tag_regex, valid_ids);
    handler.add_elevation_ = config.add_elevation;
    handler.override_values_ = config.override_values;

    const auto new_node_output = remove_extension(std::filesystem::path(config.filename.c_str()).stem()) + ".ors.new_nodes.pbf";
    osmium::io::Writer node_writer{new_node_output, header, osmium::io::overwrite::allow};
    osmium::ProgressBar progress{total_elements, osmium::isatty(2)};
    while (auto input_buffer = reader.read()) {
        osmium::memory::Buffer output_buffer{input_buffer.committed()};
        osmium::memory::Buffer new_node_output_buffer{input_buffer.committed()};
        handler.set_buffers(&output_buffer, &new_node_output_buffer);

        osmium::apply(input_buffer, handler);
        writer(std::move(output_buffer));
        node_writer(std::move(new_node_output_buffer));

        processed_elements += handler.processed_elements_;
        progress.update(processed_elements);
    }
    progress.done();
    reader.close();
    writer.close();
    node_writer.close();

    const auto mem = location_index->used_memory() / (1024UL );
    std::cout << "\nAbout " << mem << " KBytes used for node location index (in main memory or on disk).\n";

    const auto end = chrono::steady_clock::now();
    printf("\nProcessed in %.3f s\n", chrono::duration_cast<chrono::milliseconds>(end - start).count() / 1000.0);

    const auto insize = std::filesystem::file_size(config.filename);
    const auto outsize = std::filesystem::file_size(output);
    const auto reduction = insize - outsize;
    printf("\nOriginal: %20ju b\nReduced: %21lu b\nReduction: %19ju b (= %3.2f %%)\n", insize, outsize,
           reduction, static_cast<float>(reduction) / static_cast<float>(insize) * 100);
    if (config.add_elevation) {
        auto valid_nodes = valid_ids.nodes().size();
        printf("All Nodes: %19lu Nodes\n",
               valid_nodes);
        printf("Custom Elevation: %12.2f %% (%llu)\n",
               static_cast<double>(handler.nodes_with_elevation_high_precision_) /
                       static_cast<double>(valid_nodes) * 100, handler.nodes_with_elevation_high_precision_);
        printf("SRTM Elevation: %14.2f %% (%llu)\n",
               static_cast<double>(handler.nodes_with_elevation_srtm_precision_) /
                       static_cast<double>(valid_nodes) * 100, handler.nodes_with_elevation_srtm_precision_);
        printf("GMTED Elevation: %13.2f %% (%llu)\n",
               static_cast<double>(handler.nodes_with_elevation_gmted_precision_) /
                       static_cast<double>(valid_nodes) * 100, handler.nodes_with_elevation_gmted_precision_);
        printf("Failed Elevation: %12.2f %% (%llu)\n",
               static_cast<double>(handler.nodes_with_elevation_not_found_) /
                       static_cast<double>(valid_nodes) * 100,
               handler.nodes_with_elevation_not_found_);
        if (!config.override_values)
            printf("%30.2f %% already present (%llu)\n",
                   (static_cast<float>(handler.nodes_with_elevation_) / static_cast<float>(valid_nodes)) *
                           100.0,
                   handler.nodes_with_elevation_);
    }
    cout << endl;
}


