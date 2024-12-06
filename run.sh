#!/usr/bin/env bash
## Ingest NuttX Build Logs from GitHub Gist into Prometheus Pushgateway

## Any GitHub Token with read access will do:
## export GITHUB_TOKEN=...
. $HOME/github-token.sh

set -x  #  Echo commands

for (( ; ; )); do
  ## Find all defconfig files
  find $HOME/riscv/nuttx -name defconfig >/tmp/defconfig.txt

  ## Ingest logs from lupyuen/nuttx-build-log GitLab Snippets. Remove special characters.
  ## gitlab-token.sh contains "export GITHUB_TOKEN=...", any GitLab Token with read access will do.
  set +x ; . $HOME/gitlab-token.sh ; set -x
  cargo run -- \
    --user lupyuen \
    --repo nuttx-build-log \
    --defconfig /tmp/defconfig.txt \
    | tr -d '\033\007'
  GITLAB_TOKEN=
  date ; sleep 300

  ## Ingest logs from nuttxmacos GitHub Gist. Remove special characters.
  cargo run -- \
    --user nuttxmacos \
    --defconfig /tmp/defconfig.txt \
    | tr -d '\033\007'
  date ; sleep 300

  ## Ingest logs from lvanasse GitHub Gist. Remove special characters.
  cargo run -- \
    --user lvanasse \
    --defconfig /tmp/defconfig.txt \
    | tr -d '\033\007'
  date ; sleep 300

  ## Ingest logs from jerpelea GitHub Gist. Remove special characters.
  cargo run -- \
    --user jerpelea \
    --defconfig /tmp/defconfig.txt \
    | tr -d '\033\007'
  date ; sleep 300

  ## Ingest logs from nuttxpr GitHub Gist. Remove special characters.
  cargo run -- \
    --user nuttxpr \
    --defconfig /tmp/defconfig.txt \
    | tr -d '\033\007'
  date ; sleep 300

  ## Ingest logs from GitHub Actions
  # ./github.sh
  # date ; sleep 300
done
