#!/usr/bin/env bash
## Ingest NuttX Build Logs into Prometheus Pushgateway

set -x  #  Echo commands
. $HOME/github-token.sh

for (( ; ; )); do
  find $HOME/riscv/nuttx -name defconfig >/tmp/defconfig.txt

  cargo run -- --user nuttxpr  --defconfig /tmp/defconfig.txt
  date ; sleep 300

  cargo run -- --user jerpelea --defconfig /tmp/defconfig.txt
  date ; sleep 300
done
