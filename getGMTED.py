# Download GMTED data from usgs.gov
# storage required: ca. 26.3 Gb

import sys
import os.path
import urllib.request

storage_path = 'gmteddata/'
if not os.path.exists(storage_path):
    os.makedirs(storage_path)

# output = {}
lng_limit = -180
lat_limit = -70
requested_files = 0
existing_files = 0
downloaded_files = 0
for lng_index in range(12):
    for lat_index in range(8):

        minLng = lng_limit + 30 * lng_index
        maxLng = minLng + 30
        minLat = lat_limit + 20 * lat_index
        maxLat = minLat + 20
        lngPre = 'W' if minLng < 0 else 'E'
        latPre = 'S' if minLat < 0 else 'N'
        file = '{:02}{}{:03}{}_20101117_gmted_mea075.tif'.format(abs(minLat), latPre, abs(minLng), lngPre)

        requested_files+=1
        if not os.path.isfile(storage_path + file):
            print('downloading {}'.format(file))
            sys.stdout.flush()
            urllib.request.urlretrieve('https://edcintl.cr.usgs.gov/downloads/sciweb1/shared/topo/downloads/GMTED/Global_tiles_GMTED/075darcsec/mea/{}{:03}/{}'.format(lngPre, abs(minLng), file), storage_path + file)
            downloaded_files+=1
        else:
            existing_files+=1
            #print(storage_path + '{}.tif'.format(file) + ' exists already')
        # if os.path.isfile(storage_path + file):
        #     output[file] = {'minLng': minLng, 'maxLng': maxLng, 'minLat': minLat, 'maxLat': maxLat}

print("\n {} files downloaded of {} ({} files already present)".format(downloaded_files, requested_files, existing_files))
# with open(storage_path + 'tiles.json', 'w') as f:
#     json.dump(output, f, ensure_ascii=True, indent=2)
