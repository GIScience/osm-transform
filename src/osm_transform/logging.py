import logging
import sys

from src.osm_transform import logger


class CustomFormatter(logging.Formatter):
    grey = "\x1b[38;20m"
    yellow = "\x1b[33;20m"
    red = "\x1b[31;20m"
    bold_red = "\x1b[31;1m"
    reset = "\x1b[0m"
    underline = "\x1b[4m"
    format_string = "%(asctime)s - %(name)s - %(levelname)s - %(message)s (%(filename)s:%(lineno)d)"  # type: ignore

    FORMATS = {
        logging.DEBUG: grey + underline + format_string + reset,
        logging.INFO: grey + format_string + reset,
        logging.WARNING: yellow + format_string + reset,
        logging.ERROR: red + format_string + reset,
        logging.CRITICAL: bold_red + format_string + reset,
    }

    def format(self, record):  # type: ignore
        log_fmt = self.FORMATS.get(record.levelno)
        formatter = logging.Formatter(log_fmt)
        return formatter.format(record)


def initialize_logging(level: str = "info") -> None:
    correct_level = logging.getLevelName(level)
    logger.setLevel(correct_level)

    stdout_handler = logging.StreamHandler(sys.stdout)
    stdout_handler.setLevel(correct_level)
    stdout_handler.setFormatter(CustomFormatter())

    # file_handler = logging.FileHandler("logs.log")
    # file_handler.setLevel(correct_level)
    # file_handler.setFormatter(CustomFormatter())

    # logger.addHandler(file_handler)
    logger.addHandler(stdout_handler)
