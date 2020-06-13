#!/bin/bash

set -ex

docker build . -t trangarbot:latest
docker tag trangarbot trangar.azurecr.io/trangarbot
az acr login -n trangar
docker push trangar.azurecr.io/trangarbot
