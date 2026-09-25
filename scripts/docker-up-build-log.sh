#!/bin/sh

docker compose -f compose-dev.yaml up flaresolverr byparr trawl -d
docker compose -f compose-dev.yaml down flaresolverr-aggregate
docker compose -f compose-dev.yaml up flaresolverr-aggregate --build -d
docker logs flaresolverr-aggregate -f
