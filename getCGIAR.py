# Download CGIAR data from cgiar.org and unpack tifs
# storage required: ca. 63.1 Gb

import sys
import os.path
from io import BytesIO
from zipfile import ZipFile
import urllib.request
import urllib.error
from contextlib import closing

storage_path = 'srtmdata/'
if not os.path.exists(storage_path):
    os.makedirs(storage_path)

# output = {}
# lng_limit = -180
# lat_limit = 60
# for lng_index in range(72):
#     for lat_index in range(24):
#         minLng = lng_limit + 5 * lng_index
#         maxLng = minLng + 5
#         maxLat = lat_limit - 5 * lat_index
#         minLat = maxLat - 5
#         file = 'srtm_{:02}_{:02}'.format(lng_index + 1, lat_index + 1)
tilesfile = open("cgiar_tiles", "r")
requested_files = 0
existing_files = 0
downloaded_files = 0
for line in tilesfile:
    requested_files += 1
    file = line.strip()
    if not os.path.isfile(storage_path + '{}.tif'.format(file)):
        try:
            with closing(urllib.request.urlopen(
                    'http://srtm.csi.cgiar.org/wp-content/uploads/files/srtm_5x5/TIFF/{}.zip'.format(file))) as url:
                print('downloading {}.tif '.format(file))
                sys.stdout.flush()
                with ZipFile(BytesIO(url.read())) as zip:
                    zip.extract('{}.tif'.format(file), storage_path)
            downloaded_files += 1
        except urllib.error.HTTPError as e:
            print('Unexpected error retrieving {}.zip'.format(file))
            continue
        except urllib.error.URLError as e:
            print('Error {} downloading {}.zip'.format(e, file))
            continue
    else:
        existing_files += 1
        # print(storage_path + '{}.tif'.format(file) + ' exists already')
    # if os.path.isfile(storage_path + '{}.tif'.format(file)):
    #     output[file] = {'minLng': minLng, 'maxLng': maxLng, 'minLat': minLat, 'maxLat': maxLat}
tilesfile.close()
print(
    "\n {} files downloaded of {} ({} files already present)".format(downloaded_files, requested_files, existing_files))
