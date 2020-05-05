#include <string>
#include <iostream>

#include "gdal.h"
#include "gdal_priv.h"
#include "cpl_conv.h"
#include "gdal_utils.h"

using namespace std;

double getElevation(double lat, double lng, bool debug = false) {
  int lngIndex = static_cast<int>(-180 + floor((180 + lng) / 30) * 30);
  int latIndex = static_cast<int>(-70 + floor((70 + lat) / 20) * 20);
  char lngPre = lngIndex < 0 ? 'W' : 'E';
  char latPre = latIndex < 0 ? 'S' : 'N';

  char pszFilename[100];
  sprintf(pszFilename, "../gmteddata/%02d%c%03d%c_20101117_gmted_mea075.tif", abs(latIndex), latPre, abs(lngIndex), lngPre);
  if (debug)
    printf("Filename for coordinates %.6f - %.6f : %s\n", lng, lat, pszFilename);

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

  int iPixel = static_cast<int>(floor(adfInvGeoTransform[0] + adfInvGeoTransform[1] * lng + adfInvGeoTransform[2] * lat));
  int iLine = static_cast<int>(floor(adfInvGeoTransform[3] + adfInvGeoTransform[4] * lng + adfInvGeoTransform[5] * lat));
  if (debug)
    printf( "Coordinates: %.6f %.6f\n", lat, lng);
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
  cout << getElevation(-9.111483, 148.758735, true) << endl;

  // // 337.0 (339)
  // cout << getElevation(49.949784, 11.57517, true) << endl;
  // // 453.0 (438)
  // cout << getElevation(49.967668, 11.575127, true) << endl;
  // // 447.0 (432)
  // cout << getElevation(49.967682, 11.574842, true) << endl;
  //
  // // 3131 (3169)
  // cout << getElevation(-22.532854, -65.112474, true) << endl;
  //
  // // 123 (124)
  // cout << getElevation(38.065392, -87.099609, true) << endl;
  //
  // // 1615 (1615)
  // cout << getElevation(40, -105.2277023, true) << endl;
  // // (1618)
  // cout << getElevation(39.99899999, -105.2277023, true) << endl;
  // cout << getElevation(39.9989999, -105.2277023, true) << endl;
  // // 1617 (1618)
  // cout << getElevation(39.998999, -105.2277023, true) << endl;
  //
  // // 1046 (1070)
  // cout << getElevation(47.467668, 14.575127, true) << endl;
  // // 1113 (1115)
  // cout << getElevation(47.467753, 14.573911, true) << endl;
  //
  // // 1946 (1990)
  // cout << getElevation(46.468835, 12.578777, true) << endl;
  //
  // // 845 (841)
  // cout << getElevation(48.469123, 9.576393, true) << endl;
  //
}
