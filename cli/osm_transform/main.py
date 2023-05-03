from __future__ import annotations

import os
import time
from dataclasses import dataclass
from .docs import app as docs_app

from typing import Optional

import typer

from . import __app_name__, __version__, logger
from .logging.logging import initialize_logging, LogLevel

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


@app.command()
def foo() -> int:
    logger.info("#################################")
    logger.info("########## Foo Start ############")
    logger.info("INFO: Successfully ran foo command")
    logger.debug("DEBUG: Successfully ran foo command")
    logger.error("ERROR: Successfully ran foo command")
    logger.info("#################################")
    return 0


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
    ctx.obj = Shared(cpu_count=cores)
    return
