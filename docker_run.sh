#!/bin/bash
docker rm -f  rusty-rust-transformer || true
docker build -t  rusty-rust-transformer:local .
docker run --rm -it -v .:/osm --name rusty-rust-transformer --user "$(id -u):$(id -g)" rusty-rust-transformer:local $@