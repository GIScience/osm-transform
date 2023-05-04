from __future__ import annotations

import os
import time
from dataclasses import dataclass
from .docs import app as docs_app

from typing import Optional

import typer

from . import __app_name__, __version__, logger
from .logging.logging import initialize_logging, LogLevel
from .srtm_data import download_all, process_x_y_info
from .gmted_data import download_all

app = typer.Typer()

app.add_typer(docs_app, name="docs", help="Generate documentation")
script_start_time = time.time()

cpu_count: int | None = os.cpu_count()

if cpu_count is None or cpu_count == 1:
    cpu_count = 1
else:
    cpu_count = cpu_count - 1


def _version_callback(value: bool) -> None:
    if value:
        typer.echo(f"{__app_name__} v{__version__}")
        raise typer.Exit()


@dataclass
class Shared:
    cpu_count: int


@app.command(help="Foo example implementation")
def foo() -> int:
    logger.info("#################################")
    logger.info("########## Foo Start ############")
    logger.info("INFO: Successfully ran foo command")
    logger.debug("DEBUG: Successfully ran foo command")
    logger.error("ERROR: Successfully ran foo command")
    logger.info("#################################")
    return 0


@app.command(help="Download SRTM CGIAR tiles")
def srtm_download(
    x: Optional[int] = typer.Option(None, help="Column ID of the CGIAR tile to download. All by default", min=1, max=72),
    y: Optional[int] = typer.Option(None, help="Row ID of the CGIAR tile to download. All by default", min=1, max=24)
) -> None:

    feedback = {
        "requested": 0,
        "existing": 0,
        "downloaded": 0
    }

    if not x and not y:
        logger.info('Downloading all valid CGIAR tiles')
        download_all(feedback)

    elif x and y:
        logger.info('Downloading single CGIAR tile')
        try:
            process_x_y_info(x, y, feedback)
        except:
            # Error logged in srtm_data.download_tile()
            pass
    elif x and not y:
        logger.info('Column set. Not implemented')
    elif y and not x:
        logger.info('Row set.  Not implemented')
    logger.info(f"Files downloaded {feedback['downloaded']} of {feedback['requested']} ({feedback['existing']} files already present)")


@app.command(help="Download GMTED tiles")
def gmted_download() -> None:
    feedback = {
        "requested": 0,
        "existing": 0,
        "downloaded": 0
    }
    logger.info('Downloading all GMTED tiles')
    download_all(feedback)
    logger.info(f"Files downloaded {feedback['downloaded']} of {feedback['requested']} ({feedback['existing']} files already present)")


@app.callback()
def main(
        ctx: typer.Context,
        logging: LogLevel = LogLevel.INFO,
        cores: int = typer.Option(cpu_count - 1 if cpu_count else 1,
                                  help="Set the number of cores to use for processing."),
        version: Optional[bool] = typer.Option(
            None,
            "--version",
            "-v",
            help="Show the application's version and exit.",
            callback=_version_callback,
            is_eager=True,
        ),
) -> None:
    if logging is None:
        logging = LogLevel.INFO
    initialize_logging(logging)
    logger.info("############ Run info ############")
    logger.info(f"Log level: {logging}")
    logger.info(f"Number of cores: {cores}")
    logger.info("############ End info ############")
    ctx.obj = Shared(cpu_count=cores)
    return
