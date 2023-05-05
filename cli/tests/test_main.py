import os

from typer.testing import CliRunner

from cli.osm_transform import __app_name__, __version__, main
from osm_transform.srtm_data import STORAGE_FOLDER

runner = CliRunner()


def test_version() -> None:
    result = runner.invoke(main.app, ["--version"])
    assert result.exit_code == 0
    assert f"{__app_name__} v{__version__}\n" in result.stdout


def test_foo():
    result = runner.invoke(main.app, ["--logging=debug", "foo"])
    assert result.exit_code == 0


def test_srtm_download():
    srtm_file_path = STORAGE_FOLDER / 'srtm_01_02.tif'

    if srtm_file_path.exists():
        os.remove(srtm_file_path)

    assert srtm_file_path.exists() is False

    result = runner.invoke(main.app, ["--logging=debug", "srtm-download", "--x=1", "--y=2"])
    assert result.exit_code == 0
    assert "INFO - Files downloaded 1 of 1 (0 files already present)" in result.stdout.split('\n')[-2]
    assert srtm_file_path.exists() is True

    result = runner.invoke(main.app, ["--logging=debug", "srtm-download", "--x=1", "--y=2"])
    assert result.exit_code == 0
    assert "INFO - Files downloaded 0 of 1 (1 files already present)" in result.stdout.split('\n')[-2]

    result = runner.invoke(main.app, ["--logging=debug", "srtm-download", "--x=1", "--y=1"])
    assert result.exit_code == 0
    assert "is not a valid tile." in result.stdout.split('\n')[-3]
    assert "INFO - Files downloaded 0 of 1 (0 files already present)" in result.stdout.split('\n')[-2]
