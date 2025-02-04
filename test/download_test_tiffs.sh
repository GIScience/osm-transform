#!/usr/bin/env bash

curl -O https://edcintl.cr.usgs.gov/downloads/sciweb1/shared/topo/downloads/GMTED/Global_tiles_GMTED/075darcsec/mea/E000/50N000E_20101117_gmted_mea075.tif
curl -O https://edcintl.cr.usgs.gov/downloads/sciweb1/shared/topo/downloads/GMTED/Global_tiles_GMTED/075darcsec/mea/E000/30N000E_20101117_gmted_mea075.tif

curl -O https://srtm.csi.cgiar.org/wp-content/uploads/files/srtm_5x5/TIFF/srtm_38_02.zip
curl -O https://srtm.csi.cgiar.org/wp-content/uploads/files/srtm_5x5/TIFF/srtm_38_03.zip

unzip -o srtm_38_02.zip
unzip -o srtm_38_03.zip

read -p "delete unused stuff? [y|N] " -n 1 choice
if [[ "$choice" =~ [yY] ]]; then
  rm -f readme.txt srtm_38_02.hdr srtm_38_02.tfw srtm_38_02.zip srtm_38_03.hdr srtm_38_03.tfw srtm_38_03.zip
fi
