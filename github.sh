#!/usr/bin/env bash
## Ingest NuttX Build Logs from GitHub Actions into Prometheus Pushgateway

## Any GitHub Token with read access will do:
## export GITHUB_TOKEN=...
. $HOME/github-token.sh

set -e  #  Exit when any command fails
set -x  #  Echo commands

user=NuttX
repo=nuttx
linux_step=7  ## TODO: Step may change for Linux Builds
msys2_step=9  ## TODO: Step may change for msys2 Builds

function ingest_log {
  ## Fetch the Jobs for the Run ID. Get the Job ID for the Job Name.
  local os=$1 ## "Linux" or "msys2"
  local step=$2 ## "7" or "9"
  local group=$3 ## "arm-01"
  local job_name="$os ($group)"
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
  sleep 1

  ## log_file looks like /tmp/ingest-nuttx-builds/Linux (arm-01)/7_Run builds.txt
  ## Or /tmp/ingest-nuttx-builds/msys2 (msys2)/9_Run Builds.txt
  ## filename looks like ci-arm-01.log
  ## pathname looks like /tmp/ingest-nuttx-builds/ci-arm-01.log
  ## url looks like https://github.com/NuttX/nuttx/actions/runs/11603561928/job/32310817851#step:7:83
  local log_file="$tmp_path/$os ($group)/${step}_Run builds.txt"
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
    --nuttx-hash $nuttx_hash \
    --apps-hash $apps_hash \
    --group $group \
    --run-id $run_id \
    --job-id $job_id \
    --step $step
}

## Loop forever until the Latest Run ID for today has completed
for (( ; ; )); do

  ## Generate the list of deconfigs
  defconfig=/tmp/defconfig-github.txt
  find $HOME/riscv/nuttx -name defconfig >$defconfig

  ## Get the Latest Completed Run ID for today
  date=$(date -u +'%Y-%m-%d')
  run_id=$(
    gh run list \
      --repo $user/$repo \
      --limit 1 \
      --created $date \
      --status completed \
      --json databaseId,name,displayTitle,conclusion \
      --jq '.[].databaseId'
  )
  echo run_id=$run_id
  if [[ "$run_id" == "" ]]; then
    echo No completed runs for today, waiting...
    sleep 300
    continue
  fi

  ## Get the Latest Run ID for today. Check that it has completed.
  latest_run_id=$(
    gh run list \
      --repo $user/$repo \
      --limit 1 \
      --created $date \
      --json databaseId,name,displayTitle,conclusion \
      --jq '.[].databaseId'
  )
  echo latest_run_id=$latest_run_id
  if [[ "$run_id" != "$latest_run_id" ]]; then
    echo Latest run has not completed, waiting...
    sleep 300
    continue
  fi

  ## For Testing
  ## run_id=11603561928
  ## job_id=32310817851

  ## Find the Second-Last Commit Hash for NuttX Mirror Repo
  ## Because the Last Commit is always "Enable macOS and Windows Builds"
  ## TODO: This might change
  tmp_path=/tmp/ingest-nuttx-builds-nuttx
  rm -rf $tmp_path
  mkdir $tmp_path
  pushd $tmp_path
  git clone https://github.com/NuttX/nuttx
  cd nuttx
  last_two_commits=$(git log -2 --pretty=format:"%H")
  echo last_two_commits=$last_two_commits
  nuttx_hash=$(echo $last_two_commits | cut -d ' ' -f 2)
  echo nuttx_hash=$nuttx_hash
  popd

  ## Find the Last Commit Hash for NuttX Apps Repo
  ## TODO: NuttX Apps Repo may have changed when we run this. Find the hash from the GitHub Log instead.
  rm -rf $tmp_path
  mkdir $tmp_path
  pushd $tmp_path
  git clone https://github.com/apache/nuttx-apps
  cd nuttx-apps
  apps_hash=$(git log -1 --pretty=format:"%H")
  echo apps_hash=$apps_hash
  popd

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
  ## TODO: Handle macOS when the warnings have been cleaned up
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
    xtensa-01 xtensa-02 \
    msys2
  do
    ## Ingest the Log File
    if [[ "$group" == "msys2" ]]; then
      ingest_log "msys2" $msys2_step $group
    else
      ingest_log "Linux" $linux_step $group
    fi
  done

  ## Run once only
  break
done
