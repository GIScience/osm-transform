#include <string>
#include <iostream>

#include "gdal.h"
#include "gdal_priv.h"
#include "cpl_conv.h"
#include "gdal_utils.h"

using namespace std;

double getElevation(double lat, double lng, bool debug = false) {
  // lng = (double)((int)(lng * 1e7) / 1e7);
  // lat = (double)((int)(lat * 1e7) / 1e7);
  if (debug)
    printf("Original coordinates %.7f - %.7f\n", lng, lat);
  int lngIndex = floor(1 + (180 + lng) / 5);
  double latIndexDouble = 1 + (60 - lat) / 5;
  int latIndex = floor(latIndexDouble);
  // if (abs(latIndexDouble - latIndex) < 1 / 1e7 / 5) {
  //   latIndex--;
  // }
  char pszFilename[100];
  sprintf(pszFilename, "cgiardata/srtm_%02d_%02d.tif", lngIndex, latIndex);
  if (debug)
    printf("Filename for coordinates %.7f - %.7f : %s\n", lng, lat, pszFilename);

  GDALDataset  *poDataset;
  GDALAllRegister();
  poDataset = (GDALDataset*)GDALOpen(pszFilename, GA_ReadOnly);
  if(poDataset == NULL) {
    if (debug)
      cout << "Failed to read input data, existing." << endl;
    return 0;
  }
  if (debug)
    printf( "Dataset opened. (format: %s; size: %d x %d x %d)\n", poDataset->GetDriver()->GetDescription(), poDataset->GetRasterXSize(), poDataset->GetRasterYSize(), poDataset->GetRasterCount());

  double adfGeoTransform[6];
  double adfInvGeoTransform[6];
  if(poDataset->GetGeoTransform(adfGeoTransform) != CE_None) {
    if (debug)
      cout << "Failed to get transformation from input data." << endl;
    return 0;
  }
  if (!GDALInvGeoTransform(adfGeoTransform, adfInvGeoTransform)) {
    if (debug)
      cout << "Failed to get reverse transformation." << endl;
    return 0;
  }
  // int pixelOffset = adfInvGeoTransform[0];
  // int lineOffset = adfInvGeoTransform[3];
  int iPixel = static_cast<int>(floor(adfInvGeoTransform[0] + adfInvGeoTransform[1] * lng + adfInvGeoTransform[2] * lat));
  int iLine = static_cast<int>(floor(adfInvGeoTransform[3] + adfInvGeoTransform[4] * lng + adfInvGeoTransform[5] * lat));
  if (debug)
    printf( "Coordinates: %.7f %.7f\n", lat, lng);
  if (debug)
    printf( "Image coordinates: %d %d\n", iPixel, iLine);

  double adfPixel[2];
  if (poDataset->GetRasterBand(1)->RasterIO(GF_Read, iPixel, iLine, 1, 1, adfPixel, 1, 1, GDT_CFloat64, 0, 0) != CE_None) {
    if (debug)
      cout << "Failed to read data at coordinates." << endl;
    return 0;
  }
  return adfPixel[0];
}

int main (int argc, char** argv) {
  // Node: 4353241011
  cout << getElevation(48.6456794, 8.2324622, true) << endl;
  // cout << getElevation(-9.111483, 148.758735, true) << endl;
  // cout << getElevation(49.949784, 11.57517, true) << endl; //337
  // cout << getElevation(49.968668, 11.575127, true) << endl; //466
  // cout << getElevation(49.968682, 11.574842, true) << endl; //455
  // cout << getElevation(-22.532854, -65.110474, true) << endl; //3134
  // cout << getElevation(38.065392, -87.099609, true) << endl; //120
  // cout << getElevation(40, -105.2277023, true) << endl; //1615
  // cout << getElevation(39.99999999, -105.2277023, true) << endl; //1615
  // cout << getElevation(39.9999999, -105.2277023, true) << endl; //1615
  // cout << getElevation(39.999999, -105.2277023, true) << endl; //1616
  // cout << getElevation(47.468668, 14.575127, true) << endl; //986
  // cout << getElevation(47.467753, 14.573911, true) << endl; //1091
  // cout << getElevation(46.468835, 12.578777, true) << endl; //1951
  // cout << getElevation(48.469123, 9.576393, true) << endl; //841
  // cout << getElevation(56.4787319, 17.6118363, true) << endl; // NaN
}
