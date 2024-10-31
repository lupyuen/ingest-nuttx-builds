#!/usr/bin/env bash
## Ingest NuttX Build Logs from GitHub Actions into Prometheus Pushgateway

## Any GitHub Token with read access will do:
## export GITHUB_TOKEN=...
. $HOME/github-token.sh

set -e  #  Exit when any command fails
set -x  #  Echo commands

user=NuttX
repo=nuttx
step=7  ## TODO: Step may change

function ingest_log {
  ## Fetch the Jobs for the Run ID. Get the Job ID for the Job Name.
  local group=$1 ## "arm-01"
  local job_name="Linux ($group)"
  local job_id=$(
    curl -L \
      -H "Accept: application/vnd.github+json" \
      -H "Authorization: Bearer $GITHUB_TOKEN" \
      -H "X-GitHub-Api-Version: 2022-11-28" \
      https://api.github.com/repos/$user/$repo/actions/runs/$run_id/jobs?per_page=100 \
      | jq ".jobs | map(select(.name == \"$job_name\")) | .[].id"
      # | jq ".jobs | map(select(.id == $job_id)) | .[].name"
      # | jq ".jobs[].id,.jobs[].name"
  )
  echo job_id=$job_id
  if [[ "$job_id" == "" ]]; then
    echo Job ID missing for Run ID $run_id, Job Name $job_name
    sleep 10
    return
  fi
  sleep 10

  ## filename looks like ci-arm-01.log
  ## pathname looks like /tmp/ingest-nuttx-builds/ci-arm-01.log
  ## url looks like https://github.com/NuttX/nuttx/actions/runs/11603561928/job/32310817851#step:7:83
  local log_file="$tmp_path/Linux ($group)/${step}_Run builds.txt"
  local filename="ci-$group.log"
  local pathname="$tmp_path/$filename"

  ## For Testing
  local linenum=83
  local url="https://github.com/$user/$repo/actions/runs/$run_id/job/$job_id#step:$step:$linenum"

  ## Remove the Timestamp Column
  cat "$log_file" \
    | colrm 1 29 \
    > $pathname
  head -n 100 $pathname
  echo url=$url

  ## Ingest the Log File
  cargo run -- \
    --user $user \
    --repo $repo \
    --defconfig $defconfig \
    --file $pathname \
    --group $group \
    --run-id $run_id \
    --job-id $job_id \
    --step $step
}

## Generate the list of deconfigs
defconfig=/tmp/defconfig.txt
find $HOME/riscv/nuttx -name defconfig >$defconfig

## Get the latest Run ID for today
date=$(date -u +'%Y-%m-%d')
run_id=$(
  gh run list \
    --repo $user/$repo \
    --limit 1 \
    --created $date \
    --json databaseId,name,displayTitle,conclusion \
    --jq '.[].databaseId'
)
echo run_id=$run_id
if [[ "$run_id" == "" ]]; then
  echo Quitting, no runs for today
  sleep 10
  exit
fi

## For Testing
## run_id=11603561928
## job_id=32310817851

## Download the Run Logs
## https://docs.github.com/en/rest/actions/workflow-runs?apiVersion=2022-11-28#download-workflow-run-logs
curl -L \
  --output /tmp/run-log.zip \
  -H "Accept: application/vnd.github+json" \
  -H "Authorization: Bearer $GITHUB_TOKEN" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  https://api.github.com/repos/$user/$repo/actions/runs/$run_id/logs

## Unzip the Log Files
tmp_path=/tmp/ingest-nuttx-builds
rm -rf $tmp_path
mkdir $tmp_path
pushd $tmp_path
unzip /tmp/run-log.zip
popd

## For All Target Groups
## TODO: macOS, msvc, msys2
for group in \
  arm-01 arm-02 arm-03 arm-04 \
  arm-05 arm-06 arm-07 arm-08 \
  arm-09 arm-10 arm-11 arm-12 \
  arm-13 arm-14 \
  arm64-01 \
  other \
  risc-v-01 risc-v-02 risc-v-03 risc-v-04 \
  risc-v-05 risc-v-06 \
  sim-01 sim-02 sim-03 \
  x86_64-01 \
  xtensa-01 xtensa-02
do
  ingest_log $group
done
