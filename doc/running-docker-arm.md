# armv7-2
docker run -v .:/tmp/foo --rm --platform linux/arm/v7 -it arm32v7/debian:stable-backports /bin/bash

# aarch64
docker run -v .:/tmp/foo --rm --platform linux/arm64 -it arm64v8/debian:stable-backports /bin/bash
