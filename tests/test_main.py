from typer.testing import CliRunner

from osm_transform import main
from src.osm_transform import __app_name__, __version__

runner = CliRunner()


def test_version() -> None:
    result = runner.invoke(main.app, ["--version"])
    assert result.exit_code == 0
    assert f"{__app_name__} v{__version__}\n" in result.stdout


def test_foo():
    result = runner.invoke(main.app, ["--logging=debug", "foo"])
    assert result.exit_code == 0
