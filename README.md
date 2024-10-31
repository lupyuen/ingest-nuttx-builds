# Ingest Build Logs from Apache NuttX RTOS into Prometheus Pushgateway

Read the article...

- ["Your very own Build Farm for Apache NuttX RTOS"](https://lupyuen.codeberg.page/articles/ci2.html)

To ingest NuttX Build Logs into Prometheus Pushgateway: [run.sh](run.sh)

```bash
## Any GitHub Token with read access will do:
## export GITHUB_TOKEN=...
. $HOME/github-token.sh
find $HOME/nuttx -name defconfig >/tmp/defconfig.txt
cargo run -- --user nuttxpr  --defconfig /tmp/defconfig.txt
cargo run -- --user jerpelea --defconfig /tmp/defconfig.txt
```

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
- [highlights.json](highlights.json)
- [error-builds.json](error-builds.json)
- [success-builds.json](success-builds.json)

Remember to check for suspicious activity!

```bash
tail -f /opt/homebrew/var/log/grafana/grafana.log \
  | grep -v /api/live/ws
```
