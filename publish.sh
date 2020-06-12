#!/bin/bash

set -ex

docker build . -t trangarbot:latest
docker tag trangarbot trangar.azurecr.io/trangarbot
docker push trangar.azurecr.io/trangarbot
