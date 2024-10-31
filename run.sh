#!/usr/bin/env bash
## Ingest NuttX Build Logs into Prometheus Pushgateway

set -x  #  Echo commands
. $HOME/github-token.sh

for (( ; ; )); do
  find $HOME/riscv/nuttx -name defconfig >/tmp/defconfig.txt

  cargo run -- \
    --user NuttX \
    --repo nuttx \
    --defconfig /tmp/defconfig.txt \
    --file /tmp/ingest-nuttx-builds/ci-arm-01.log \
    --group arm-01 \
    --run-id 11603561928 \
    --job-id 32310817851 \
    --step 7
  exit

  cargo run -- --user nuttxpr  --defconfig /tmp/defconfig.txt
  date ; sleep 300

  cargo run -- --user jerpelea --defconfig /tmp/defconfig.txt
  date ; sleep 300
done
