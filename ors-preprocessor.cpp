#include <string>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <chrono>

#include <boost/regex.hpp>
#include <boost/unordered_set.hpp>
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
#include <libgen.h>
#include <libconfig.h++>

using namespace std;

bool file_exists(const string& filename) {
  ifstream ifile(filename.c_str());
  return (bool)ifile;
}

int filesize(const string& filename) {
    ifstream in(filename.c_str(), std::ifstream::ate | std::ifstream::binary);
    return (int)in.tellg();
}

string remove_extension(const string& filename) {
    size_t lastdot = filename.find_first_of(".");
    if (lastdot == string::npos) return filename;
    return filename.substr(0, lastdot);
}

class FirstPassHandler : public osmium::handler::Handler {
  friend ostream& operator<<(ostream& out, const FirstPassHandler& ce);
  const set<string> invalidating_tags{"building", "landuse"};

  boost::regex* remove_tags;
  bool first = true;

  bool validating_tags(const string& tag, const string& value) {
    if (tag == "highway") return true;
    if (tag == "route") return true;
    if (tag == "railway" && value == "platform") return true;
    if (tag == "public_transport" && value == "platform") return true;
    if (tag == "man_made" && value == "pier") return true;
    return false;
  }

  bool check_tags(const osmium::TagList& tags) {
    int tag_count = 0;
    bool is_removable = false;
    for (const osmium::Tag& tag : tags) {
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

    unsigned long long node_count = 0;
    unsigned long long valid_node_count = 0;
    unsigned long long relation_count = 0;
    unsigned long long valid_relation_count = 0;
    unsigned long long way_count = 0;
    unsigned long long valid_way_count = 0;

    boost::unordered_set<long long> valid_nodes;
    boost::unordered_set<long long> valid_ways;
    boost::unordered_set<long long> valid_relations;

    bool DEBUG_NO_FILTER = false;
    bool ONLY_COUNT = false;

    void init(boost::regex* re, unsigned long long i_nodes_reserve, unsigned long long i_ways_reserve, unsigned long long i_rels_reserve, bool debug_no_filter, bool only_count) {
      remove_tags = re;
      DEBUG_NO_FILTER = debug_no_filter;
      ONLY_COUNT = only_count;
      if (!only_count) {
        valid_nodes.reserve(i_nodes_reserve);
        valid_ways.reserve(i_ways_reserve);
        valid_relations.reserve(i_rels_reserve);
      }
    }

    void node (const osmium::Node& node) {
      node_count++;
    }

    void way (const osmium::Way& way) {
      way_count++;
      if (DEBUG_NO_FILTER || way.nodes().size() < 2 || check_tags(way.tags())) {
        return;
      }
      for (const osmium::NodeRef& n : way.nodes()) {
        valid_node_count++;
        if (!ONLY_COUNT)
          valid_nodes.emplace(n.ref());
      }
      valid_way_count++;
      if (!ONLY_COUNT)
        valid_ways.emplace(way.id());
    }

    void relation (const osmium::Relation& rel) {
      relation_count++;
      if (DEBUG_NO_FILTER || check_tags(rel.tags())) {
        return;
      }
      for (const auto& member : rel.members()) {
        if (member.type() == osmium::item_type::node) {
          valid_node_count++;
          if (!ONLY_COUNT)
            valid_nodes.emplace(member.ref());
        }
      }
      valid_relation_count++;
      if (!ONLY_COUNT)
        valid_relations.emplace(rel.id());
    }
};

class RewriteHandler : public osmium::handler::Handler {
  friend ostream& operator<<(ostream& out, const RewriteHandler& ce);
  osmium::memory::Buffer* m_buffer;
  boost::unordered_set<long long>* valid_nodes;
  boost::unordered_set<long long>* valid_ways;
  boost::unordered_set<long long>* valid_relations;
  boost::regex* remove_tags;
  bool DEBUG_NO_FILTER = false;
  bool DEBUG_NO_TAG_FILTER = false;

  void copy_tags(osmium::builder::Builder& parent, const osmium::TagList& tags) {
    osmium::builder::TagListBuilder builder{parent};
    for (const auto& tag : tags) {
      total_tags++;
      if (DEBUG_NO_TAG_FILTER || !boost::regex_match(tag.key(), *remove_tags)) {
        valid_tags++;
        builder.add_tag(tag);
      }
    }
  }

  public:

    unsigned long long valid_elements = 0;
    unsigned long long processed_elements = 0;
    unsigned long long total_tags = 0;
    unsigned long long valid_tags = 0;

    void set_buffer(osmium::memory::Buffer* buffer) {
      m_buffer = buffer;
      valid_elements = 0;
      processed_elements = 0;
      total_tags = 0;
      valid_tags = 0;
    }

    void init(boost::regex* re, boost::unordered_set<long long>* i_valid_nodes, boost::unordered_set<long long>* i_valid_ways, boost::unordered_set<long long>* i_valid_relations, bool debug_no_filter, bool debug_no_tag_filter) {
      remove_tags = re;
      valid_nodes = i_valid_nodes;
      valid_ways = i_valid_ways;
      valid_relations = i_valid_relations;
      valid_elements = valid_nodes->size() + valid_ways->size() + valid_relations->size();
      DEBUG_NO_FILTER = debug_no_filter;
      DEBUG_NO_TAG_FILTER = debug_no_tag_filter;
    }

    void node(const osmium::Node& node) {
      processed_elements++;
      {
        if (DEBUG_NO_FILTER || valid_nodes->count(node.id()) > 0) {
          osmium::builder::NodeBuilder builder{*m_buffer};
          builder.set_id(node.id());
          builder.set_location(node.location());
          copy_tags(builder, node.tags());
        }
      }
      m_buffer->commit();
    }

    void way(const osmium::Way& way) {
      processed_elements++;
      {
        if (DEBUG_NO_FILTER || valid_ways->count(way.id()) > 0) {
          osmium::builder::WayBuilder builder{*m_buffer};
          builder.set_id(way.id());
          builder.add_item(way.nodes());
          copy_tags(builder, way.tags());
        }
      }
      m_buffer->commit();
    }

    void relation(const osmium::Relation& relation) {
      processed_elements++;
      {
        if (DEBUG_NO_FILTER || valid_relations->count(relation.id()) > 0) {
          osmium::builder::RelationBuilder builder{*m_buffer};
          builder.set_id(relation.id());
          builder.add_item(relation.members());
          copy_tags(builder, relation.tags());
        }
      }
      m_buffer->commit();
    }
};

ostream& operator<<(ostream& out, const FirstPassHandler& handler) {
  return out  << "valid nodes: " << handler.valid_nodes.size() << " (" << handler.node_count << "), "
              << "valid ways: " << handler.valid_ways.size() << " (" << handler.way_count << "), "
              << "valid relations: " << handler.valid_relations.size() << " (" << handler.relation_count << ")";
}

ostream& operator<<(ostream& out, const RewriteHandler& handler) {
  return out  << "valid elements: " << handler.valid_elements << " (" << handler.processed_elements << "), "
              << "valid tags: " << handler.valid_tags << " (" << handler.total_tags << ")";
}

int main (int argc, char** argv) {
  if (argc < 2 || !file_exists(argv[1])) {
    cerr << "Usage: " << argv[0] << " [OSM file] [-s]" << endl;
    return 1;
  }

  string remove_tag_regex_str = "REMOVE NO TAGS";
  unsigned long long nodes_reserve = 1;
  unsigned long long ways_reserve = 1;
  unsigned long long rels_reserve = 1;
  bool debug_output = false;
  bool debug_no_filter = false;
  bool debug_no_tag_filter = false;

  // TODO: implement actual flag checking...
  bool only_count = argc >= 3;

  try {
    libconfig::Config cfg;
    cfg.readFile("ors-preprocessor.cfg");
    const libconfig::Setting& root = cfg.getRoot();
    root.lookupValue("remove_tag", remove_tag_regex_str);
    root.lookupValue("nodes_reserve", nodes_reserve);
    root.lookupValue("ways_reserve", ways_reserve);
    root.lookupValue("rels_reserve", rels_reserve);
    root.lookupValue("debug_output", debug_output);
    root.lookupValue("debug_no_filter", debug_no_filter);
    root.lookupValue("debug_no_tag_filter", debug_no_tag_filter);
    if (debug_no_filter) {
      cout << "DEBUG MODE: Filtering disabled" << endl << endl;
    }
    if (debug_no_tag_filter) {
      cout << "DEBUG MODE: Tag filtering disabled" << endl << endl;
    }
  } catch(const libconfig::FileIOException& fioex) {
    std::cerr << "I/O error while reading file." << std::endl;
    return 2;
  } catch(const libconfig::ParseException &pex) {
    std::cerr << "Parse error at " << pex.getFile() << ":" << pex.getLine()
              << " - " << pex.getError() << std::endl;
    return 2;
  } catch (const libconfig::SettingNotFoundException &nfex) {
    cerr << "Missing setting in configuration file: " << nfex.what() << endl;
    return 2;
  }

  try {
    boost::regex remove_tag_regex(remove_tag_regex_str, boost::regex::icase);
    auto start = chrono::steady_clock::now();
    cout << "Processing first pass: validate ways & relations..." << endl;

    osmium::io::Reader reader{argv[1]};
    long unsigned int insize = reader.file_size();
    osmium::ProgressBar progress{insize, osmium::isatty(2)};
    FirstPassHandler first_pass;
    first_pass.init(&remove_tag_regex, nodes_reserve, ways_reserve, rels_reserve, debug_no_filter, only_count);
    if (!only_count)
      printf("Preallocating: %llu nodes, %llu ways, %llu relations\n", nodes_reserve, ways_reserve, rels_reserve);
    while (osmium::memory::Buffer input_buffer = reader.read()) {
      osmium::apply(input_buffer, first_pass);
      progress.update(reader.offset());
    }
    progress.done();
    progress.remove();
    reader.close();
    if (only_count) {
      cout  << "valid nodes: " << first_pass.valid_node_count << " (" << first_pass.node_count << "), "
                  << "valid ways: " << first_pass.valid_way_count << " (" << first_pass.way_count << "), "
                  << "valid relations: " << first_pass.valid_relation_count << " (" << first_pass.relation_count << ")\n\nWARNING: valid node count is an estimate and most likely higher than the actual number!\n";
    } else {
      cout << first_pass << endl;
    }
    auto end = chrono::steady_clock::now();
    printf("Processed in %.3f s\n", chrono::duration_cast<chrono::milliseconds>(end - start).count() / 1000.0);
    cout << endl;
    if (only_count) {
      return 0;
    }

    string output = remove_extension(basename(argv[1])) + ".ors.pbf";
    unsigned long long total_elements = first_pass.node_count + first_pass.way_count + first_pass.relation_count;
    unsigned long long processed_elements = 0;
    unsigned long long processed_nanos = 0;

    start = chrono::steady_clock::now();
    cout << "Processing second pass: rebuild data..." << endl;
    osmium::io::Reader second_reader{argv[1]};
    osmium::io::Header header;
    header.set("generator", "ORS Proprocessor v1.0");
    osmium::io::Writer writer{output, header, osmium::io::overwrite::allow};
    RewriteHandler handler;
    handler.init(&remove_tag_regex, &first_pass.valid_nodes, &first_pass.valid_ways, &first_pass.valid_relations, debug_no_filter, debug_no_tag_filter);

    while (osmium::memory::Buffer input_buffer = second_reader.read()) {
      auto step_start = chrono::steady_clock::now();

      int bytes_per_cycle = input_buffer.committed();
      osmium::memory::Buffer output_buffer{input_buffer.committed()};
      handler.set_buffer(&output_buffer);
      osmium::apply(input_buffer, handler);
      writer(move(output_buffer));

      auto step_end = chrono::steady_clock::now();
      processed_elements += handler.processed_elements;
      processed_nanos += chrono::duration_cast<chrono::nanoseconds>(step_end - step_start).count();
      printf("\rProgress: %llu / %llu (%3.2f %%)", processed_elements, total_elements, ((float)processed_elements / total_elements) * 100.0d);
      if (debug_output) {
        printf(" - Average element process time: %.3f ms - bytes / cycle: %d, %llu elements / cycle", processed_nanos / processed_elements / 1000.0d, bytes_per_cycle, handler.processed_elements);
      }
      fflush(stdout);
    }
    second_reader.close();
    writer.close();
    end = chrono::steady_clock::now();
    printf("\nProcessed in %.3f s\n", chrono::duration_cast<chrono::milliseconds>(end - start).count() / 1000.0);

    int outsize = filesize(output);
    int reduction = insize - outsize;
    printf("\nOriginal: %15lu b\nReduced: %16d b\nReduction: %14d b (= %3.2f %%)\n\n", insize, outsize, reduction, (float) reduction / insize * 100);

  } catch (const exception& e) {
    cerr << e.what() << '\n';
    return(3);
  }
  return 0;
}
