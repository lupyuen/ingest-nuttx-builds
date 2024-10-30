# Ingest Build Logs from Apache NuttX RTOS into Prometheus Pushgateway

Read the article...
- ["Your very own Build Farm for Apache NuttX RTOS"](https://lupyuen.codeberg.page/articles/ci2.html)

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
