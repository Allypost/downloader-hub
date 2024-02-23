#!/usr/bin/env sh
set -ex

# Update the packages
apk upgrade -U
echo "[$(date -R)] Packages updated" >> /tmp/update-packages.log