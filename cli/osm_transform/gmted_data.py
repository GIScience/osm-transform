import os
import urllib
from pathlib import Path

from osm_transform import logger

STORAGE_FOLDER = Path(__file__).resolve().parents[2] / 'gmteddata'
if not os.path.exists(STORAGE_FOLDER):
    os.makedirs(STORAGE_FOLDER)

lng_limit = -180
lat_limit = -70


def gmted_exists(name: str) -> bool:
    return Path(STORAGE_FOLDER / name).exists()


def download_all(feedback):
    for lng_index in range(12):
        for lat_index in range(8):

            min_lng = lng_limit + 30 * lng_index
            min_lat = lat_limit + 20 * lat_index
            lng_pre = 'W' if min_lng < 0 else 'E'
            lat_pre = 'S' if min_lat < 0 else 'N'
            file = '{:02}{}{:03}{}_20101117_gmted_mea075.tif'.format(abs(min_lat), lat_pre, abs(min_lng), lng_pre)

            feedback["requested"] += 1
            if not gmted_exists(file):
                logger.info(f'Downloading {file}')
                urllib.request.urlretrieve(
                    f'https://edcintl.cr.usgs.gov/downloads/sciweb1/shared/topo/downloads/GMTED/Global_tiles_GMTED'
                    f'/075darcsec/mea/{lng_pre}{abs(min_lng):03}/{file}',
                    STORAGE_FOLDER / file)
                feedback["downloaded"] += 1
            else:
                feedback["existing"] += 1
