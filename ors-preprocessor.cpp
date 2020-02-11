#include <string>
#include <boost/filesystem.hpp>
#include <fstream>
#include <iostream>
#include <chrono>
#include <unordered_map>

#include <boost/regex.hpp>
#include <boost/unordered_set.hpp>

#include "gdal.h"
#include "gdal_priv.h"
#include "gdal_utils.h"
#include "cpl_conv.h"

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
typedef unsigned long long llu;
typedef vector<int> vi;

void setBit(vi &A, llu k) {
  if (k < 0) return;
  A[k/32] |= 1 << (k%32);  // Set the bit at the k-th position in A
}

bool testBit(vector<int> &A, llu k) {
  if (k < 0) return 0;
  return (A[k/32] & (1 << (k%32))) != 0;
}

llu countBits(vector<int> &A) {
  llu count = 0;
  for (auto &intval : A) {
    count += __builtin_popcount(intval);
  }
  return count;
}

bool file_exists(const string& filename) {
  ifstream ifile(filename.c_str());
  return (bool)ifile;
}

llu filesize(const string& filename) {
    ifstream in(filename.c_str(), std::ifstream::ate | std::ifstream::binary);
    return (llu)in.tellg();
}

string remove_extension(const string& filename) {
    size_t lastdot = filename.find_first_of(".");
    if (lastdot == string::npos) return filename;
    return filename.substr(0, lastdot);
}

class MaxIDHandler : public osmium::handler::Handler {
  public:
    llu node_max_id = 0;
    llu way_max_id = 0;
    llu relation_max_id = 0;

    void node(const osmium::Node& node) {
      if (node.id() < 0) return;
      node_max_id = node.id() > node_max_id ? node.id() : node_max_id;
    }

    void way(const osmium::Way& way) {
      if (way.id() < 0) return;
      way_max_id = way.id() > way_max_id ? way.id() : way_max_id;
    }

    void relation(const osmium::Relation& rel) {
      if (rel.id() < 0) return;
      relation_max_id = rel.id() > relation_max_id ? rel.id() : relation_max_id;
    }
};

class FirstPassHandler : public osmium::handler::Handler {
  friend ostream& operator<<(ostream& out, const FirstPassHandler& ce);
  const set<string> invalidating_tags{"building", "landuse"};
  boost::regex* remove_tags;

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

    llu node_count = 0;
    llu relation_count = 0;
    llu way_count = 0;

    vector<int>* valid_nodes;
    vector<int>* valid_ways;
    vector<int>* valid_relations;

    bool DEBUG_NO_FILTER = false;

    void init(boost::regex* re, vector<int>* i_valid_nodes, vector<int>* i_valid_ways, vector<int>* i_valid_relations, bool debug_no_filter) {
      remove_tags = re;
      valid_nodes = i_valid_nodes;
      valid_ways = i_valid_ways;
      valid_relations = i_valid_relations;
      DEBUG_NO_FILTER = debug_no_filter;
    }

    void node(const osmium::Node& node) {
      node_count++;
    }

    void way(const osmium::Way& way) {
      way_count++;
      if (DEBUG_NO_FILTER || way.id() < 0 || way.nodes().size() < 2 || check_tags(way.tags())) {
        return;
      }
      for (const osmium::NodeRef& n : way.nodes()) {
        setBit(*valid_nodes, n.ref());
      }
      setBit(*valid_ways, way.id());
    }

    void relation (const osmium::Relation& rel) {
      relation_count++;
      if (DEBUG_NO_FILTER || rel.id() < 0 || check_tags(rel.tags())) {
        return;
      }
      for (const auto& member : rel.members()) {
        if (member.type() == osmium::item_type::node) {
          setBit(*valid_nodes, member.ref());
        }
      }
      setBit(*valid_relations, rel.id());
    }
};

class RewriteHandler : public osmium::handler::Handler {
  friend ostream& operator<<(ostream& out, const RewriteHandler& ce);
  osmium::memory::Buffer* m_buffer;
  vector<int>* valid_nodes;
  vector<int>* valid_ways;
  vector<int>* valid_relations;
  boost::regex* remove_tags;
  bool DEBUG_NO_FILTER = false;
  bool DEBUG_NO_TAG_FILTER = false;
  static const int NO_DATA_VALUE = -32768;
  unordered_map<string, GDALDataset*> elevationData;

  void copy_tags(osmium::builder::Builder& parent, const osmium::TagList& tags, int ele = NO_DATA_VALUE) {
    osmium::builder::TagListBuilder builder{parent};
    if (ele > NO_DATA_VALUE) {
      builder.add_tag("ele", to_string(ele));
    }
    for (const auto& tag : tags) {
      total_tags++;
      if (DEBUG_NO_TAG_FILTER || !boost::regex_match(tag.key(), *remove_tags)) {
        valid_tags++;
        builder.add_tag(tag);
      }
    }
  }

  double getElevationCGIAR(double lat, double lng, bool debug = false) {
    int lngIndex = static_cast<int>(floor((180 + lng) / 5)) + 1;
    int latIndex = static_cast<int>(floor((59.9999999 - lat) / 5)) + 1;
    char pszFilename[100];
    sprintf(pszFilename, "srtmdata/srtm_%02d_%02d.tif", lngIndex, latIndex);
    if (debug)
      printf("Filename for coordinates %.6f - %.6f : %s\n", lng, lat, pszFilename);
    return getElevationFromFile(lat, lng, pszFilename, debug);
  }

  double getElevationGMTED(double lat, double lng, bool debug = false) {
    int lngIndex = static_cast<int>(-180 + floor((180 + lng) / 30) * 30);
    int latIndex = static_cast<int>(-70 + floor((70 + lat) / 20) * 20);
    char lngPre = lngIndex < 0 ? 'W' : 'E';
    char latPre = latIndex < 0 ? 'S' : 'N';
    char pszFilename[100];
    sprintf(pszFilename, "gmteddata/%02d%c%03d%c_20101117_gmted_mea075.tif", abs(latIndex), latPre, abs(lngIndex), lngPre);
    if (debug)
      printf("Filename for coordinates %.6f - %.6f : %s\n", lng, lat, pszFilename);
    return getElevationFromFile(lat, lng, pszFilename, debug);
  }

  double getElevationFromFile(double lat, double lng, char* pszFilename, bool debug = false) {
    GDALDataset  *poDataset;
    auto search = elevationData.find(pszFilename);
    if (search != elevationData.end()) {
      poDataset = elevationData.at(pszFilename);
    } else {
      if (!file_exists(pszFilename)) {
        if (debug)
          cout << "File does not exist: " << pszFilename << endl;
        return NO_DATA_VALUE;
      }
      poDataset = (GDALDataset*)GDALOpen(pszFilename, GA_ReadOnly);
      elevationData.insert(make_pair(pszFilename, poDataset));
      if(poDataset == NULL) {
        if (debug)
          cout << "Failed to read input data, existing." << endl;
        return NO_DATA_VALUE;
      }
      if (debug)
        printf( "Dataset opened. (format: %s; size: %d x %d x %d)\n", poDataset->GetDriver()->GetDescription(), poDataset->GetRasterXSize(), poDataset->GetRasterYSize(), poDataset->GetRasterCount());
    }
    double adfGeoTransform[6];
    double adfInvGeoTransform[6];
    if(poDataset->GetGeoTransform(adfGeoTransform) != CE_None) {
      if (debug)
        cout << "Failed to get transformation from input data." << endl;
        return NO_DATA_VALUE;
    }
    if (!GDALInvGeoTransform(adfGeoTransform, adfInvGeoTransform)) {
      if (debug)
        cout << "Failed to get reverse transformation." << endl;
        return NO_DATA_VALUE;
    }
    int iPixel = static_cast<int>(floor(adfInvGeoTransform[0] + adfInvGeoTransform[1] * lng + adfInvGeoTransform[2] * lat));
    int iLine = static_cast<int>(floor(adfInvGeoTransform[3] + adfInvGeoTransform[4] * lng + adfInvGeoTransform[5] * lat));

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

    if (iPixel > poDataset->GetRasterXSize() || iLine > poDataset->GetRasterYSize()) {
     printf( "Coordinates: %.6f %.6f\n", lat, lng);
     printf( "Image coordinates: %d %d\n", iPixel, iLine);
    }
    if (debug) {
      printf( "Coordinates: %.6f %.6f\n", lat, lng);
      printf( "Image coordinates: %d %d\n", iPixel, iLine);
    }

    double adfPixel[2];
    if (poDataset->GetRasterBand(1)->RasterIO(GF_Read, iPixel, iLine, 1, 1, adfPixel, 1, 1, GDT_CFloat64, 0, 0) != CE_None) {
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

    void set_buffer(osmium::memory::Buffer* buffer) {
      m_buffer = buffer;
      valid_elements = 0;
      processed_elements = 0;
      total_tags = 0;
      valid_tags = 0;
    }

    void init(boost::regex* re, vector<int>* i_valid_nodes, vector<int>* i_valid_ways, vector<int>* i_valid_relations, bool debug_no_filter, bool debug_no_tag_filter) {
      remove_tags = re;
      valid_nodes = i_valid_nodes;
      valid_ways = i_valid_ways;
      valid_relations = i_valid_relations;
      valid_elements = valid_nodes->size() + valid_ways->size() + valid_relations->size();
      DEBUG_NO_FILTER = debug_no_filter;
      DEBUG_NO_TAG_FILTER = debug_no_tag_filter;
      GDALAllRegister();
    }

    void node(const osmium::Node& node) {
      processed_elements++;
      if (node.id() < 0) return;
      {
        if (DEBUG_NO_FILTER || testBit(*valid_nodes, node.id()) > 0) {
          osmium::builder::NodeBuilder builder{*m_buffer};
          builder.set_id(node.id());
          builder.set_location(node.location());
          int ele = NO_DATA_VALUE;
          if (addElevation && !node.tags().has_key("ele")) {
            ele = getElevationCGIAR(node.location().lat(), node.location().lon());
            if (ele == NO_DATA_VALUE) {
              ele = getElevationGMTED(node.location().lat(), node.location().lon());
            }
          }
          copy_tags(builder, node.tags(), ele);
        }
      }
      m_buffer->commit();
    }

    void way(const osmium::Way& way) {
      processed_elements++;
      if (way.id() < 0) return;
      {
        if (DEBUG_NO_FILTER || testBit(*valid_ways, way.id()) > 0) {
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

ostream& operator<<(ostream& out, const FirstPassHandler& handler) {
  return out  << "valid nodes: " << countBits(*handler.valid_nodes) << " (" << handler.node_count << "), "
              << "valid ways: " << countBits(*handler.valid_ways) << " (" << handler.way_count << "), "
              << "valid relations: " << countBits(*handler.valid_relations) << " (" << handler.relation_count << ")";
}

ostream& operator<<(ostream& out, const RewriteHandler& handler) {
  return out  << "valid elements: " << handler.valid_elements << " (" << handler.processed_elements << "), "
              << "valid tags: " << handler.valid_tags << " (" << handler.total_tags << ")";
}

int main (int argc, char** argv) {
  char* filename;
  bool doMemoryCheck = false;
  bool addElevation = false;
  for (char **arg = argv; *arg; ++arg) {
    if (strcmp(*arg, "-c") == 0) {
      doMemoryCheck = true;
    } else if (strcmp(*arg, "-e") == 0) {
      addElevation = true;
    } else {
      filename = *arg;
    }
  }
  if (!file_exists(filename)) {
    cerr << "Usage: " << argv[0] << "[-e] [-c] [OSM file]" << endl;
    return 1;
  }

  string remove_tag_regex_str = "REMOVE NO TAGS";
  bool debug_output = false;
  bool debug_no_filter = false;
  bool debug_no_tag_filter = false;
  llu nodes_max_id;
  llu ways_max_id;
  llu rels_max_id;
  try {
    libconfig::Config cfg;
    cfg.readFile("ors-preprocessor.cfg");
    const libconfig::Setting& root = cfg.getRoot();
    root.lookupValue("remove_tag", remove_tag_regex_str);
    root.lookupValue("nodes_max_id", nodes_max_id);
    root.lookupValue("ways_max_id", ways_max_id);
    root.lookupValue("rels_max_id", rels_max_id);
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
    if (doMemoryCheck) {
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

      cout << "Max IDs: Node " << maxIDHandler.node_max_id << " Way " << maxIDHandler.way_max_id << " Relation " << maxIDHandler.relation_max_id << endl;
      if (maxIDHandler.node_max_id > nodes_max_id || maxIDHandler.way_max_id > ways_max_id || maxIDHandler.relation_max_id > rels_max_id) {
        cout << "Insufficient memory settings." << endl;
        return 1;
      }
    }

    boost::regex remove_tag_regex(remove_tag_regex_str, boost::regex::icase);
    printf("Allocating memory: %llu (%.2f Mb) nodes, %llu (%.2f Mb) ways, %llu (%.2f Mb) relations\n", nodes_max_id / 32 + 1, nodes_max_id / (1024*1024*8.0), ways_max_id / 32 + 1, ways_max_id / (1024*1024*8.0), rels_max_id / 32 + 1, rels_max_id / (1024*1024*8.0));
    vector<int> valid_nodes(nodes_max_id / 32 + 1, 0);
    vector<int> valid_ways(ways_max_id / 32 + 1, 0);
    vector<int> valid_relations(rels_max_id / 32 + 1, 0);

    cout << "Processing first pass: validate ways & relations..." << endl;
    auto start = chrono::steady_clock::now();
    osmium::io::Reader first_pass_reader{filename};
    llu insize = first_pass_reader.file_size();
    osmium::ProgressBar progress{insize, osmium::isatty(2)};
    FirstPassHandler first_pass;
    first_pass.init(&remove_tag_regex, &valid_nodes, &valid_ways, &valid_relations, debug_no_filter);
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

    string output = remove_extension(basename(argv[1])) + ".ors.pbf";
    llu total_elements = first_pass.node_count + first_pass.way_count + first_pass.relation_count;
    llu processed_elements = 0;
    llu processed_nanos = 0;

    start = chrono::steady_clock::now();
    cout << "Processing second pass: rebuild data..." << endl;
    osmium::io::Reader second_reader{filename};
    osmium::io::Header header;
    header.set("generator", "ORS Proprocessor v1.0");
    osmium::io::Writer writer{output, header, osmium::io::overwrite::allow};
    RewriteHandler handler;
    handler.init(&remove_tag_regex, first_pass.valid_nodes, first_pass.valid_ways, first_pass.valid_relations, debug_no_filter, debug_no_tag_filter);
    handler.addElevation = addElevation;

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

    llu outsize = filesize(output);
    llu reduction = insize - outsize;
    printf("\nOriginal: %20llu b\nReduced: %21llu b\nReduction: %19llu b (= %3.2f %%)\n\n", insize, outsize, reduction, (float) reduction / insize * 100);

  } catch (const exception& e) {
    cerr << e.what() << '\n';
    return(3);
  }
  return 0;
}
