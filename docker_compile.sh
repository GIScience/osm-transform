#!/usr/bin/env sh
docker build -t ors-preprocessor -f Dockerfile .
docker run -i -v $(pwd):/src ors-preprocessor
docker container prune -f
docker image rm ors-preprocessor:latest
