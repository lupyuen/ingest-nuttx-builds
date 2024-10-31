#!/usr/bin/env bash
## Ingest NuttX Build Logs from GitHub Gist into Prometheus Pushgateway

## Any GitHub Token with read access will do:
## export GITHUB_TOKEN=...
. $HOME/github-token.sh

set -x  #  Echo commands

for (( ; ; )); do
  find $HOME/riscv/nuttx -name defconfig >/tmp/defconfig.txt

  ## Ingest logs from GitHub Gist
  cargo run -- --user nuttxpr  --defconfig /tmp/defconfig.txt
  date ; sleep 300

  ## Ingest logs from GitHub Gist
  cargo run -- --user jerpelea --defconfig /tmp/defconfig.txt
  date ; sleep 300

  ## Ingest logs from GitHub Actions
  # ./github.sh
  # date ; sleep 300
done
