#!/usr/bin/env bash
## Show Logs from Grafana

set -x  #  Echo commands

clear && tail -f /opt/homebrew/var/log/grafana/grafana.log \
  | grep --line-buffered "logger=context " \
  | grep --line-buffered -v "path=/api/frontend-metrics " \
  | grep --line-buffered -v "path=/api/live/ws " \
  | grep --line-buffered -v "path=/api/plugins/grafana-lokiexplore-app/settings " \
  | grep --line-buffered -v "path=/api/user/auth-tokens/rotate " \
  | grep --line-buffered -v "path=/favicon.ico " \
  | cut -d ' ' -f 9-15
