#!/usr/bin/env bash
## Ingest NuttX Build Logs from GitHub Actions into Prometheus Pushgateway

. $HOME/github-token.sh
set -x  #  Echo commands

user=NuttX
repo=nuttx
step=7  ## TODO: Step may change

defconfig=/tmp/defconfig.txt
find $HOME/riscv/nuttx -name defconfig >$defconfig

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

group=arm-01

## For Testing
## run_id=11603561928
## job_id=32310817851

## Fetch the Jobs for the Run ID. Get the Job ID for the Job Name.
job_name="Linux ($group)"
job_id=$(
  curl -L \
    -H "Accept: application/vnd.github+json" \
    -H "Authorization: Bearer $GITHUB_TOKEN" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    https://api.github.com/repos/$user/$repo/actions/runs/$run_id/jobs \
    | jq ".jobs | map(select(.name == \"$job_name\")) | .[].id"
    # | jq ".jobs | map(select(.id == $job_id)) | .[].name"
    # | jq ".jobs[].id,.jobs[].name"
)
echo job_id=$job_id

## Download the Run Logs
## https://docs.github.com/en/rest/actions/workflow-runs?apiVersion=2022-11-28#download-workflow-run-logs
curl -L \
  --output /tmp/run-log.zip \
  -H "Accept: application/vnd.github+json" \
  -H "Authorization: Bearer $GITHUB_TOKEN" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  https://api.github.com/repos/$user/$repo/actions/runs/$run_id/logs

## filename looks like ci-arm-01.log
## pathname looks like /tmp/ingest-nuttx-builds/ci-arm-01.log
## url looks like https://github.com/NuttX/nuttx/actions/runs/11603561928/job/32310817851#step:7:83
tmp_path=/tmp/ingest-nuttx-builds
log_file="$tmp_path/Linux ($group)/${step}_Run builds.txt"
filename="ci-$group.log"
pathname="$tmp_path/$filename"
linenum=83
url="https://github.com/$user/$repo/actions/runs/$run_id/job/$job_id#step:$step:$linenum"

rm -rf $tmp_path
mkdir $tmp_path
pushd $tmp_path
unzip /tmp/run-log.zip
popd

cat "$log_file" \
  | colrm 1 29 \
  > $pathname
head -n 100 $pathname
echo url=$url

cargo run -- \
  --user $user \
  --repo $repo \
  --defconfig $defconfig \
  --file $pathname \
  --group $group \
  --run-id $run_id \
  --job-id $job_id \
  --step $step
