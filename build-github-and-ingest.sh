#!/usr/bin/env bash
## Build NuttX Mirror Repo and Ingest NuttX Build Logs
## from GitHub Actions into Prometheus Pushgateway

set -e  #  Exit when any command fails
set -x  #  Echo commands

## TODO: Go to NuttX Mirror Repo (github.com/NuttX/nuttx), click Sync Fork > Discard Commits

## Enable macOS and Windows Builds
~/nuttx-release/enable-macos-windows.sh

## Wait for build to start
sleep 300

## Wait for buld to complete and ingest the logs
./github.sh
