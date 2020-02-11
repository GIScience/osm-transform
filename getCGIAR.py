import sys
import os.path
from io import BytesIO
from zipfile import ZipFile
import json
import urllib2
from contextlib import closing

storage_path = 'srtmdata/'
output = {}
lng_limit = -180
lat_limit = 60
for lng_index in range(72):
    for lat_index in range(24):

        minLng = lng_limit + 5 * lng_index
        maxLng = minLng + 5
        maxLat = lat_limit - 5 * lat_index
        minLat = maxLat - 5
        file = 'srtm_{:02}_{:02}'.format(lng_index + 1, lat_index + 1)
        sys.stdout.write('.')
        sys.stdout.flush()
        if not os.path.isfile(storage_path + '{}.tif'.format(file)):
            try:
                with closing(urllib2.urlopen('http://srtm.csi.cgiar.org/wp-content/uploads/files/srtm_5x5/TIFF/{}.zip'.format(file))) as url:
                    print('downloading {}.tif '.format(file))
                    with ZipFile(BytesIO(url.read())) as zip:
                        zip.extract('{}.tif'.format(file), storage_path)
            except urllib2.HTTPError, e:
                #print('{} @ http://srtm.csi.cgiar.org/wp-content/uploads/files/srtm_5x5/TIFF/{}.zip'.format(e.code, file))
                continue;
            except urllib2.URLError, e:
                print ('Unexpected error retrieving {}.zip'.format(file))
                continue;
        else:
            pass #print(storage_path + '{}.tif'.format(file) + ' exists already')

        if os.path.isfile(storage_path + '{}.tif'.format(file)):
            output[file] = {'minLng': minLng, 'maxLng': maxLng, 'minLat': minLat, 'maxLat': maxLat}

with open(storage_path + 'tiles.json', 'w') as f:
    json.dump(output, f, ensure_ascii=True, indent=2)
