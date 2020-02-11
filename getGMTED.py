import sys
import sys
import os.path
import json
import urllib

storage_path = 'gmteddata/'
output = {}
lng_limit = -180
lat_limit = -70
for lng_index in range(12):
    for lat_index in range(8):

        minLng = lng_limit + 30 * lng_index
        maxLng = minLng + 30
        minLat = lat_limit + 20 * lat_index
        maxLat = minLat + 20
        lngPre = 'W' if minLng < 0 else 'E'
        latPre = 'S' if minLat < 0 else 'N'

        file = '{:02}{}{:03}{}_20101117_gmted_mea075.tif'.format(abs(minLat), latPre, abs(minLng), lngPre)
        sys.stdout.write('.')
        sys.stdout.flush()
        if not os.path.isfile(storage_path + file):
            print('downloading {}'.format(file))
            urllib.urlretrieve('https://edcintl.cr.usgs.gov/downloads/sciweb1/shared/topo/downloads/GMTED/Global_tiles_GMTED/075darcsec/mea/{}{:03}/{}'.format(lngPre, abs(minLng), file), storage_path + file)
        else:
            pass #print(storage_path + '{}.tif'.format(file) + ' exists already')

        if os.path.isfile(storage_path + file):
            output[file] = {'minLng': minLng, 'maxLng': maxLng, 'minLat': minLat, 'maxLat': maxLat}

with open(storage_path + 'tiles.json', 'w') as f:
    json.dump(output, f, ensure_ascii=True, indent=2)
