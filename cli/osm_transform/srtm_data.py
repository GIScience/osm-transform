from __future__ import annotations

import os
import urllib.request
from contextlib import closing
from io import BytesIO
from pathlib import Path
from urllib.error import HTTPError, URLError
from zipfile import ZipFile

from osm_transform import logger

data = {
    1: [2, 7, 12, 15, 16, 17, 18, 19, 21], 2: [1, 2, 7, 13, 14, 15, 16, 17], 3: [1, 2, 8, 9, 15, 16],
    4: [1, 2, 8, 11, 12, 14, 15, 16], 5: [1, 2, 8, 9, 12, 14, 16, 17], 6: [1, 9, 13, 14, 15, 16, 17],
    7: [1, 15, 16, 17],
    8: [1, 14, 15, 16, 17, 18], 9: [1, 14, 15, 16, 17], 10: [1, 2, 17, 18], 11: [1, 2, 3, 17],
    12: [1, 2, 3, 4, 5, 6, 17],
    13: [1, 2, 3, 4, 5, 6, 7, 8], 14: [1, 2, 3, 4, 5, 6, 7, 8, 9], 15: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 18],
    16: [1, 2, 3, 4, 5, 6, 7, 8, 9], 17: [1, 2, 3, 4, 5, 6, 7, 8, 9], 18: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13],
    19: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13], 20: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 18, 19],
    21: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 21, 22, 23],
    22: [1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24],
    23: [1, 2, 3, 4, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24],
    24: [1, 2, 3, 4, 6, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 23],
    25: [1, 2, 3, 4, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 23], 26: [3, 11, 12, 13, 14, 15, 16, 17, 18, 19],
    27: [1, 12, 13, 14, 15, 16, 17, 18], 28: [1, 13, 14, 15, 16, 17], 29: [13, 14, 15, 16, 23], 30: [5, 13, 14, 24],
    31: [5, 9, 17], 32: [5, 9, 10], 33: [6, 7, 8, 9, 10], 34: [1, 2, 7, 8, 9, 10, 11, 14, 20, 21],
    35: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16, 21], 36: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
    37: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 23], 38: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
    39: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18],
    40: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19],
    41: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19],
    42: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19],
    43: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19],
    44: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 22],
    45: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 17, 18],
    46: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18],
    47: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14, 15, 16, 22],
    48: [1, 2, 3, 4, 5, 6, 7, 8, 9, 13, 14, 15, 16, 17],
    49: [1, 2, 3, 4, 5, 6, 7, 16], 50: [1, 2, 3, 4, 5, 6, 7, 8, 22, 23],
    51: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 22, 23], 52: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 20],
    53: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11], 54: [1, 2, 3, 4, 5, 6, 7, 8, 9],
    55: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
    56: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15], 57: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    58: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    59: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 17, 18, 19],
    60: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
    61: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19],
    62: [1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19],
    63: [1, 2, 3, 4, 5, 6, 7, 8, 11, 12, 13, 14, 15, 16, 17, 18, 19],
    64: [1, 2, 3, 4, 5, 6, 8, 10, 11, 13, 14, 15, 16, 17, 18, 19, 20],
    65: [1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 13, 14, 15, 16, 17, 18, 19, 20, 21],
    66: [1, 3, 4, 8, 9, 10, 11, 13, 14, 15, 16, 17, 18, 19, 20, 21],
    67: [1, 2, 3, 8, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20], 68: [1, 2, 3, 11, 13, 14, 15, 16, 17, 19, 23, 24],
    69: [1, 2, 10, 11, 14, 15, 16, 17], 70: [1, 2, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 21, 22, 23],
    71: [2, 10, 11, 12, 13, 15, 16, 17, 19, 20, 21, 22], 72: [2, 13, 14, 15, 16, 20, 21, 22]
}

STORAGE_FOLDER = Path(__file__).resolve().parents[2] / 'srtmdata'
if not os.path.exists(STORAGE_FOLDER):
    os.makedirs(STORAGE_FOLDER)


def is_valid_tile(x: int, y: int) -> bool:
    if x not in data.keys():
        return False
    return y in data[x]


def get_url(x: int, y: int) -> str:
    return f'http://srtm.csi.cgiar.org/wp-content/uploads/files/srtm_5x5/TIFF/{srtm_file_name(x, y, zip=True)}'


def srtm_file_name(x: int, y: int, zip: bool = False) -> str:
    return f'srtm_{x:02}_{y:02}.{"zip" if zip else "tif"}'


def srtm_exists(x: int, y: int) -> bool:
    return Path(STORAGE_FOLDER / srtm_file_name(x, y)).exists()


def download_tile(x, y):
    try:
        with closing(urllib.request.urlopen(
                get_url(x, y))) as url:
            logger.info(f'downloading {srtm_file_name(x, y)}')
            with ZipFile(BytesIO(url.read())) as zip:
                zip.extract(f'{srtm_file_name(x, y)}', STORAGE_FOLDER)
    except HTTPError as e:
        logger.error(f'Unexpected error retrieving {srtm_file_name(x, y, True)}')
    except URLError as e:
        logger.error(f'Error {e} downloading {srtm_file_name(x, y, True)}')


def download_all(feedback):
    for x, values in data.items():
        for y in values:
            try:
                process_x_y_info(x, y, feedback)
            except:
                # Error logged in srtm_data.download_tile()
                continue


def process_x_y_info(x: int, y: int, feedback: object) -> None:
    feedback['requested'] += 1
    if not is_valid_tile(x, y):
        logger.error(f"{srtm_file_name(x, y)} is not a valid tile.")
        raise NameError
    if not srtm_exists(x, y):
        try:
            download_tile(x, y)
            feedback['downloaded'] += 1
        except Exception as e:
            # error log in download_tile function
            raise e
    else:
        feedback['existing'] += 1
