![Ingest Build Logs from Apache NuttX RTOS into Prometheus Pushgateway](https://lupyuen.github.io/images/ci4-flow.jpg)

# Ingest Build Logs from Apache NuttX RTOS into Prometheus Pushgateway

Read the articles...

- ["Continuous Integration Dashboard for Apache NuttX RTOS (Prometheus and Grafana)"](https://lupyuen.github.io/articles/ci4)

- ["Optimising the Continuous Integration for Apache NuttX RTOS (GitHub Actions)"](https://lupyuen.codeberg.page/articles/ci3.html)

- ["Your very own Build Farm for Apache NuttX RTOS"](https://lupyuen.codeberg.page/articles/ci2.html)

To ingest NuttX Build Logs into Prometheus Pushgateway: [run.sh](run.sh)

```bash
## Any GitHub Token with read access will do:
## export GITHUB_TOKEN=...
. $HOME/github-token.sh

## Find all defconfig files
find $HOME/riscv/nuttx -name defconfig >/tmp/defconfig.txt

## Ingest logs from nuttxpr GitHub Gist. Remove special characters.
cargo run -- \
  --user nuttxpr \
  --defconfig /tmp/defconfig.txt \
  | tr -d '\\033'

## Ingest logs from jerpelea GitHub Gist. Remove special characters.
cargo run -- \
  --user jerpelea \
  --defconfig /tmp/defconfig.txt \
  | tr -d '\\033'

## Ingest logs from GitHub Actions
./github.sh

## Or: Start GitHub Actions Build, wait to complete then ingest logs
./build-github-and-ingest.sh
```

[(See the __Run Log__)](https://gist.github.com/lupyuen/7da9c95b3efe39ff818772775c90da96)

To install Grafana and Prometheus...

```bash
## Install Grafana
brew install grafana
brew services start grafana
## Browse to http://localhost:3000

## Install Prometheus
brew install prometheus
brew services start prometheus
## Browse to http://localhost:9090

## Install Prometheus Pushgateway
brew install go
git clone https://github.com/prometheus/pushgateway
cd pushgateway
go run main.go &
## Browse to http://localhost:9091
```

Update the Grafana and Prometheus Configuration...
- [/opt/homebrew/etc/grafana/grafana.ini](grafana.ini)
- [/opt/homebrew/etc/prometheus.yml](prometheus.yml)

Add the Grafana Dashboard and Panels...
- [dashboard.json](dashboard.json)
  - [links.json](links.json)
  - [highlights.json](highlights.json)
  - [error-builds.json](error-builds.json)
  - [success-builds.json](success-builds.json)
- [dashboard-history.json](dashboard-history.json)
  - [history.json](history.json)

Remember to check for suspicious activity!

```bash
tail -f /opt/homebrew/var/log/grafana/grafana.log \
  | grep --line-buffered "logger=context " \
  | grep --line-buffered -v "path=/api/frontend-metrics " \
  | grep --line-buffered -v "path=/api/live/ws " \
  | grep --line-buffered -v "path=/api/plugins/grafana-lokiexplore-app/settings " \
  | grep --line-buffered -v "path=/api/user/auth-tokens/rotate " \
  | grep --line-buffered -v "path=/favicon.ico " \
  | cut -d ' ' -f 9-15
```

Highlight the HTTP Errors: iTerm > Profile > Advanced > Triggers...
- Regular Expression: `status=[4-9][^ ]+[ ]`, Action: Highlight Line, Background: Red
- Regular Expression: `path=[^ ]+[ ]`, Action: Highlight Text, Background: Dark Blue

If we see too many HTTP 404 Errors for Dubious URLs (we're not a WordPress Server!): Turn on Cloudflare > Under Attack Mode. The errors should disappear.
