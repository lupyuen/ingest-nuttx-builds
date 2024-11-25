//! Fetch the Latest Gists by User
//! Process the Build Log
//! Process each Build Target
//! Post to Prometheus Pushgateway

use std::{
    collections::HashSet, 
    fs::{self, File}, 
    io::{BufRead, BufReader}, 
    ops::Sub,
    path::Path,
    thread::sleep, 
    time::Duration, 
    vec
};
use chrono::DateTime;
use clap::Parser;
use regex::Regex;

/// Command-Line Arguments
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Owner of the GitHub Gists or GitHub Repo that will be processed (`nuttxpr` or `NuttX`)
    #[arg(long)]
    user: String,
    /// List of all defconfig pathnames, generated by: find $HOME/nuttx -name defconfig >/tmp/defconfig.txt
    #[arg(long)]
    defconfig: String,
    /// For GitHub Actions: Name of the GitHub Repo (`nuttx`)
    #[arg(long, default_value = "")]
    repo: String,
    /// For GitHub Actions: Pathname of the downloaded Run Log
    #[arg(long, default_value = "")]
    file: String,
    /// For GitHub Actions: Commit Hash of the NuttX Repo (`7f84a64109f94787d92c2f44465e43fde6f3d28f`)
    #[arg(long, default_value = "")]
    nuttx_hash: String,
    /// For GitHub Actions: Commit Hash of the NuttX Apps Repo (`d6edbd0cec72cb44ceb9d0f5b932cbd7a2b96288``)
    #[arg(long, default_value = "")]
    apps_hash: String,
    /// For GitHub Actions: Target Group of the CI Build (`arm-01`)
    #[arg(long, default_value = "")]
    group: String,
    /// For GitHub Actions: Run ID of the CI Build (`11603561928`)
    #[arg(long, default_value = "")]
    run_id: String,
    /// For GitHub Actions: Job ID for the Target Group inside the CI Build Run (`32310817851`)
    #[arg(long, default_value = "")]
    job_id: String,
    /// For GitHub Actions: Step Number of the Build Step inside the CI Job (`7`)
    #[arg(long, default_value = "")]
    step: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Init the Logger and Command-Line Args
    env_logger::init();
    let args = Args::parse();

    // If Log File is specified: Process the Log File
    if args.file != "" {
        process_file(&args).await?;
        return Ok(());
    }

    // Init the GitHub Client
    let token = std::env::var("GITHUB_TOKEN")
        .expect("GITHUB_TOKEN env variable is required");
    let octocrab = octocrab::Octocrab::builder()
        .personal_token(token)
        .build()?;

    // Fetch the Latest Gists, reverse chronological order
    let gists = octocrab
        .gists()
        .list_user_gists(&args.user)
        .per_page(50)
        .send()
        .await?;

    // Process Every Gist
    let mut past_filenames = HashSet::<String>::new();
    for mut gist in gists {
        let id = gist.id;  // "6e5150f02e081be935fa525e6546cb2b"
        let url = gist.html_url;  // "https://gist.github.com/nuttxpr/6e5150f02e081be935fa525e6546cb2b"

        // Skip the Dubious Gists
        if gist.files.first_entry().is_none() {
            println!("*** No Files: {url}");
            continue;
        }
        let file = gist.files.first_entry().unwrap();
        let filename = file.get().filename.as_str();  // "ci-arm-04.log"
        if !filename.starts_with("ci-") {
            println!("*** Not A Build Log: {url}");
            continue;
        }

        // Skip the filenames we've seen before
        // Except "ci-unknown.log" for Build Rewind
        if past_filenames.contains(filename)
            && !filename.contains("unknown") {
            println!("*** Skipping File {filename}: {url}");
            continue;
        }
        past_filenames.insert(filename.into());

        // Get the Gist URL
        let raw_url = file.get().raw_url.as_str();  // "https://gist.githubusercontent.com/nuttxpr/6e5150f02e081be935fa525e6546cb2b/raw/9f07185404c0f81914f622c0152a980022539968/ci-arm-04.log"
        let description = gist
            .description
            .unwrap_or("<No description>".into());  // "[arm-04] CI Log for nuttx @ f6facf7602003071aaabc6dd00082b7ebb2f5ab9 / nuttx-apps @ d9e178aad022030224d1c95628cab1784a13a339"
        let target_group = filename
            .replace("ci-", "")
            .replace(".log", "");  // "arm-04"
        println!("id={id} | url={url} | description={description}");
        println!("target_group={target_group:?}");
        println!("filename={filename:?}");
        println!("raw_url={raw_url:?}");

        // Description contains: [arm-14] CI Log for nuttx @ 7f84a64109f94787d92c2f44465e43fde6f3d28f / nuttx-apps @ d6edbd0cec72cb44ceb9d0f5b932cbd7a2b96288
        // Extract the NuttX Hash and the Apps Hash
        let mut nuttx_hash: Option<&str> = None;
        let mut apps_hash: Option<&str> = None;
        let re = Regex::new("nuttx @ ([0-9a-z]+) / nuttx-apps @ ([0-9a-z]+)").unwrap();
        let caps = re.captures(&description);
        if let Some(caps) = caps {
            let nuttx = caps.get(1).unwrap().as_str();
            let apps = caps.get(2).unwrap().as_str();
            nuttx_hash = Some(nuttx);  // "7f84a64109f94787d92c2f44465e43fde6f3d28f"
            apps_hash = Some(apps);  // "d6edbd0cec72cb44ceb9d0f5b932cbd7a2b96288"
        } else {
            println!("*** Missing Git Hash: {}", description);
        }
        println!("nuttx_hash={nuttx_hash:?}");
        println!("apps_hash={apps_hash:?}");
    
        // Download the Gist
        let res = reqwest::get(raw_url).await?;
        // println!("Status: {}", res.status());
        // println!("Headers:\n{:#?}", res.headers());
        let body = res.text().await?;
        // println!("Body:\n{}", body);

        // Process the Build Log
        process_log(
            &body, &args.user, &args.defconfig, &target_group, &url.as_str(), &filename,
            nuttx_hash, apps_hash,
            None, None, None, None
        ).await?;

        // Wait a while
        sleep(Duration::from_secs(1));
    }

    // Return OK
    Ok(())
}

/// Process the Log File downloaded from GitHub Actions
async fn process_file(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    assert_ne!(&args.repo, "");
    assert_ne!(&args.file, "");
    assert_ne!(&args.nuttx_hash, "");
    assert_ne!(&args.apps_hash, "");
    assert_ne!(&args.group, "");
    assert_ne!(&args.run_id, "");
    assert_ne!(&args.job_id, "");
    assert_ne!(&args.step, "");
    let filename = Path::new(&args.file)
        .file_name().unwrap()
        .to_str().unwrap();
    let log = fs::read_to_string(&args.file).unwrap();
    process_log(
        &log, &args.user, &args.defconfig, &args.group, "", filename,
        Some(&args.nuttx_hash), Some(&args.apps_hash),
        Some(&args.repo), Some(&args.run_id), Some(&args.job_id), Some(&args.step)
    ).await?;
    Ok(())
}

/// Process the Build Log, which contains Multiple Targets like:
/// ====================================================================================
/// Configuration/Tool: freedom-kl25z/nsh,CONFIG_ARM_TOOLCHAIN_GNU_EABI
/// 2024-10-30 00:43:37
/// ------------------------------------------------------------------------------------
///   Cleaning...
///   Configuring...
///   Disabling CONFIG_ARM_TOOLCHAIN_GNU_EABI
///   Enabling CONFIG_ARM_TOOLCHAIN_GNU_EABI
///   Building NuttX...
///   Normalize freedom-kl25z/nsh
/// ====================================================================================
async fn process_log(
    log: &str,  // Content of Build Log
    user: &str,  // "nuttxpr"
    defconfig: &str,  // "/tmp/defconfig.txt"
    group: &str,  // "arm-04"
    url: &str,  // "https://gist.github.com/nuttxpr/6e5150f02e081be935fa525e6546cb2b"
    filename: &str,  // "ci-arm-04.log"
    nuttx_hash: Option<&str>,  // "7f84a64109f94787d92c2f44465e43fde6f3d28f"
    apps_hash: Option<&str>,  // "d6edbd0cec72cb44ceb9d0f5b932cbd7a2b96288"
    repo: Option<&str>,  // "nuttx"
    run_id: Option<&str>,  // "11603561928"
    job_id: Option<&str>,  // "32310817851"
    step: Option<&str>,  // "7"
) -> Result<(), Box<dyn std::error::Error>> {
    // Look for the delimiter
    const DELIMITER: &str = "==========";
    let mut target_linenum: Option<usize> = None;
    let mut utc_time: Option<&str> = None;
    let mut local_time: Option<&str> = None;
    let lines = &log.split('\n').collect::<Vec<_>>();
    for (linenum, line) in lines.into_iter().enumerate() {
        // Not a delimiter: ====== test session starts
        if line.starts_with(DELIMITER) && !line.contains(" ") {
            // Process the target
            if let Some(l) = target_linenum {
                let target = &lines[l..linenum];
                process_target(
                    target, user, defconfig, group, url, filename,
                    nuttx_hash, apps_hash, utc_time, local_time,
                    repo, run_id, job_id, step, l
                ).await?;
            }
            target_linenum = Some(linenum + 1);

        // macOS Logs use Local Time, not UTC. We convert Local Time to UTC.
        // To get the UTC Time Difference:
        // Search for "utc_time=2024-11-09T03:51:42" and "local_time=2024-11-09T11:51:42"
        } else if line.starts_with("utc_time=") {
            utc_time = Some(&line[9..]);
        } else if line.starts_with("local_time=") {
            local_time = Some(&line[11..]); 
        }
    }
    Ok(())
}

/// Process a Build Target like:
/// Configuration/Tool: freedom-kl25z/nsh,CONFIG_ARM_TOOLCHAIN_GNU_EABI
/// 2024-10-30 00:43:37
/// ------------------------------------------------------------------------------------
///   Cleaning...
///   Configuring...
///   Disabling CONFIG_ARM_TOOLCHAIN_GNU_EABI
///   Enabling CONFIG_ARM_TOOLCHAIN_GNU_EABI
///   Building NuttX...
///   Normalize freedom-kl25z/nsh
async fn process_target(
    lines: &[&str],  // Content of Build Log
    user: &str,  // "nuttxpr"
    defconfig: &str,  // "/tmp/defconfig.txt"
    group: &str,  // "arm-04"
    url: &str,  // "https://gist.github.com/nuttxpr/6e5150f02e081be935fa525e6546cb2b"
    filename: &str,  // "ci-arm-04.log"
    nuttx_hash: Option<&str>,  // "7f84a64109f94787d92c2f44465e43fde6f3d28f"
    apps_hash: Option<&str>,  // "d6edbd0cec72cb44ceb9d0f5b932cbd7a2b96288"
    utc_time: Option<&str>,  // "2024-11-09T03:51:42"
    local_time: Option<&str>,  // "2024-11-09T11:51:42"
    repo: Option<&str>,  // "nuttx"
    run_id: Option<&str>,  // "11603561928"
    job_id: Option<&str>,  // "32310817851"
    step: Option<&str>,  // "7"
    linenum: usize,  // Line Number of Build Log
) -> Result<(), Box<dyn std::error::Error>> {
    println!("lines[0]={}", lines[0]);
    println!("lines.last={}", lines.last().unwrap());
    let mut l = 0;
    if lines[l].starts_with("Cmake in present:") { l += 1; }

    // Extract the Target Name "freedom-kl25z/nsh" from:
    // Configuration/Tool: freedom-kl25z/nsh,CONFIG_ARM_TOOLCHAIN_GNU_EABI
    let re = Regex::new("^Configuration/Tool: ([^,]+)").unwrap();
    let caps = re.captures(lines[l]);
    if caps.is_none() {
        println!("*** Not a target: {}", lines[l]);
        return Ok(())
    }
    let target = caps.unwrap()
        .get(1).unwrap()
        .as_str()  // "freedom-kl25z/nsh"
        .replace("/", ":");  // "freedom-kl25z:nsh"
    println!("target={target}");
    l += 1;

    // Read the Timestamp
    let mut timestamp = lines[l]
        .replace(" ", "T");
    println!("timestamp={timestamp}");
    l += 1;

    // macOS Logs use Local Time, not UTC. We convert Local Time to UTC.
    if let Some(utc_time) = utc_time {
        // Compute the Time Difference
        let local_time = local_time.unwrap();
        let utc_timestamp = format!("{}+00:00", utc_time);
        let local_timestamp = format!("{}+00:00", local_time);
        let utc_datetime = DateTime::parse_from_rfc3339(&utc_timestamp).unwrap();
        let local_datetime = DateTime::parse_from_rfc3339(&local_timestamp).unwrap();
        let timediff = local_datetime.sub(utc_datetime);
        println!("utc_time={utc_time}, local_time={local_time}, timediff={timediff}, numhours={}", timediff.num_hours());

        // Adjust the Timestamp
        let timestamp_before = format!("{}+00:00", timestamp);
        let datetime = DateTime::parse_from_rfc3339(&timestamp_before).unwrap();
        let datetime_adjust = datetime.sub(timediff);
        timestamp = datetime_adjust.to_rfc3339().as_str()[0..19].to_string();
        println!("adjusted timestamp={timestamp}");
    }

    // To Identify Errors / Warnings: Skip the known lines
    let mut msg: Vec<&str> = vec![];
    let lines = &lines[l..];
    for line in lines {
        let line = line.trim();
        if line.len() == 0 ||
            line.starts_with("----------") ||
            line.starts_with("-- ") ||  // "-- Build type:"
            line.starts_with("Cleaning") ||
            line.starts_with("Configuring") ||
            line.starts_with("Select") ||
            line.starts_with("Disabling") ||
            line.starts_with("Enabling") ||
            line.starts_with("Building") ||
            line.starts_with("Normalize") ||
            line.starts_with("% Total") ||
            line.starts_with("Dload") ||
            line.starts_with("~/apps") ||
            line.starts_with("~/nuttx") ||
            line.starts_with("find: 'boards/") ||  // "find: 'boards/risc-v/q[0-d]*': No such file or directory"
            line.starts_with("|        ^~~~~~~") ||  // `warning "FPU test not built; Only available in the flat build (CONFIG_BUILD_FLAT)"`
            line.contains("FPU test not built") ||
            line.starts_with("a nuttx-export-") ||  // "a nuttx-export-12.7.0/tools/incdir.c"
            line.contains(" PASSED") ||  // CI Test: "test_hello PASSED"
            line.contains(" SKIPPED") ||  // CI Test: "test_mm SKIPPED"
            line.contains("On branch master") ||  // "On branch master"
            line.contains("Your branch is up to date") ||  // "Your branch is up to date with 'origin/master'"
            line.contains("Changes not staged for commit") ||  // "Changes not staged for commit:"
            line.contains("git add <file>") ||  // "(use "git add <file>..." to update what will be committed)"
            line.contains("git restore <file>") ||  // "(use "git restore <file>..." to discard changes in working directory)"
            line.contains("***** BUILD FAILED") ||  // "***** BUILD FAILED"
            line.starts_with("set +") ||  // "set +e"
            line.starts_with("set -") ||  // "set -e"
            line.starts_with("+ set +") ||  // "+ set +e"
            line.starts_with("+ set -") ||  // "+ set -e"
            line.starts_with("+ res=0") ||  // "+ res=0"
            line.starts_with("Cloning into") ||  // "Cloning into 'nuttx'..."
            line.starts_with("remote: Enumerating") ||  // "remote: Enumerating objects:"
            line.starts_with("remote: Counting") ||  // "remote: Counting objects:"
            line.starts_with("remote: Compressing") ||  // "remote: Compressing objects:"
            line.starts_with("Receiving objects") ||  // "Receiving objects:   0%"
            line.starts_with("Resolving deltas") ||  // "Resolving deltas:   0%"
            line.starts_with("Updating files") ||  // "Updating files:  12%"
            line.starts_with("CPP:  ")  // "CPP:  etc/init.d/rc.sysinit"
        { continue; }

        // Skip Downloads: "100  533k    0  533k    0     0   541k      0 --:--:-- --:--:-- --:--:--  541k100 1646k    0 1646k    0     0  1573k      0 --:--:--  0:00:01 --:--:-- 17.8M"
        let re = Regex::new(r#"^[0-9]+\s+[0-9]+"#).unwrap();
        let caps = re.captures(line);
        if caps.is_some() { continue; }

        // Remember the Errors / Warnings
        println!("*** Msg: {line}");
        msg.push(line);
    }

    // Not an error:
    // "test_ltp_interfaces_aio_error_1_1 PASSED"
    // "lua-5.4.0/testes/errors.lua"
    // "nuttx-export-12.7.0/include/libcxx/__system_error"
    let msg_join = msg.join(" ");
    let contains_error = msg_join
        .replace("aio_error", "aio_e_r_r_o_r")
        .replace("errors.lua", "e_r_r_o_r_s.lua")
        .replace("_error", "_e_r_r_o_r")
        .replace("error_", "e_r_r_o_r_")
        .to_lowercase()
        .contains("error");

    // Identify CI Test as Error: "test_helloxx FAILED"
    let contains_error = contains_error ||
        msg_join.contains(" FAILED");

    // Given Board=sim, Config=rtptools
    // Identify defconfig as Error: "modified:...boards/sim/sim/sim/configs/rtptools/defconfig"
    let target_split = target.split(":").collect::<Vec<_>>();
    let board = target_split[0];
    let config = target_split[1];
    let board_config = format!("/{board}/configs/{config}/defconfig");
    let contains_error = contains_error ||
    (
        msg_join.contains(&"modified:") &&
        msg_join.contains(&"boards/") &&
        msg_join.contains(&board_config.as_str())
    );

    // Search for Warnings
    let contains_warning = msg_join
        .to_lowercase()
        .contains("warning");

    // Compute the Build Score based on Error vs Warning
    let build_score =
        if msg.is_empty() { 1.0 }
        else if contains_error { 0.0 }
        else if contains_warning { 0.5 }
        else { 0.8 };

    // Compose the Line URL: https://gist.github.com/nuttxpr/6e5150f02e081be935fa525e6546cb2b#file-ci-arm-04-log-L140
    // Based on `url``: https://gist.github.com/nuttxpr/6e5150f02e081be935fa525e6546cb2b
    // And `filename``: ci-arm-04.log
    let filename2 = filename.replace(".", "-");
    let linenum2 = linenum + l - 1;
    let url =
        if let Some(run_id) = run_id {
            assert_eq!(url, "");
            let repo = repo.unwrap();
            let job_id = job_id.unwrap();
            let step = step.unwrap();
            format!("https://github.com/{user}/{repo}/actions/runs/{run_id}/job/{job_id}#step:{step}:{linenum2}")
        }
        else { format!("{url}#file-{filename2}-L{linenum2}") };

    // Post the Target to Prometheus Pushgateway
    post_to_pushgateway(
        build_score,
        &timestamp,
        user,
        defconfig,
        group,
        &target,
        &url,
        nuttx_hash,
        apps_hash,
        &msg
    ).await?;
    Ok(())
}

// Post the Target to Prometheus Pushgateway
// cat <<EOF | curl --data-binary @- http://localhost:9091/metrics/job/nuttxpr/instance/ox64:nsh
// # TYPE build_score gauge
// # HELP build_score 1.0 for successful build, 0.0 for failed build
// build_score{ version=1, user="nuttxpr", group="risc-v-01", board="ox64", config="nsh", target="ox64:nsh", url="http://aaa", msg="warning: aaa" } 0.5
// EOF
async fn post_to_pushgateway(
    build_score: f32,
    timestamp: &str,
    user: &str,
    defconfig: &str,
    group: &str,
    target: &str,
    url: &str,
    nuttx_hash: Option<&str>,  // "7f84a64109f94787d92c2f44465e43fde6f3d28f"
    apps_hash: Option<&str>,  // "d6edbd0cec72cb44ceb9d0f5b932cbd7a2b96288"
    msg: &Vec<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get the Board and Config
    let version = 3;
    let target_split = target.split(":").collect::<Vec<_>>();
    let board = target_split[0];
    let config = target_split[1];
    let subarch = get_sub_arch(defconfig, target).await?;

    // If `group` is "risc-v-01": `arch` becomes "risc-v"
    // If `group` is "other": `arch` becomes "other"
    let arch =
        if let Some(pos) = group.rfind("-") { &group[0..pos] }
        else { group };

    // Join the messages
    let mut msg_join = msg
        .join(" \\n ")
        .replace("\"", "\\\"");

    // If messages contain CI Test "test_helloxx FAILED"
    // Then remove the non-failed messages
    if msg_join.contains(" FAILED") {
        msg_join = msg
            .iter()
            .copied()
            .filter(|s| s.contains(" FAILED"))
            .collect::<Vec<_>>()
            .join(" \\n ")
            .replace("\"", "\\\"");                
    }

    // Truncate the message to fit into Prometheus
    msg_join.truncate(512);
    let msg_opt =
        if msg.is_empty() { "".into() }
        else { format!(", msg=\"{msg_join}\"") };
    let url_display =
        if msg.is_empty() { "".into() }
        else { url.replace("https://", "") };

    // Compose the NuttX and Apps Hashes    
    let nuttx_hash_opt=
        if let Some(h) = nuttx_hash { format!(r#", nuttx_hash="{h}""#) }
        else { "".into() };
    let apps_hash_opt=
        if let Some(h) = apps_hash { format!(r#", apps_hash="{h}""#) }
        else { "".into() };

    // Tag the Build Rewind as "rewind" user
    let user =
        if group == "unknown" { "rewind" }
        else { user };

    // Compose the Pushgateway Metric
    let body = format!(
r##"
# TYPE build_score gauge
# HELP build_score 1.0 for successful build, 0.0 for failed build
build_score{{ version="{version}", timestamp="{timestamp}", user="{user}", arch="{arch}", subarch="{subarch}", group="{group}", board="{board}", config="{config}", target="{target}", url="{url}", url_display="{url_display}"{msg_opt}{nuttx_hash_opt}{apps_hash_opt} }} {build_score}
"##);
    println!("body={body}");
    let client = reqwest::Client::new();
    let pushgateway = format!("http://localhost:9091/metrics/job/{user}/instance/{target}");
    let res = client
        .post(pushgateway)
        .body(body)
        .send()
        .await?;
    println!("res={res:?}");
    if !res.status().is_success() {
        println!("*** Pushgateway Failed");
        sleep(Duration::from_secs(1));
    }
    Ok(())
}

// Given a list of all defconfig pathnames, search for a target (like "ox64:nsh")
// and return the Sub-Architecture (like "bl808")
async fn get_sub_arch(defconfig: &str, target: &str) -> Result<String, Box<dyn std::error::Error>> {
    let target_split = target.split(":").collect::<Vec<_>>();
    let board = target_split[0];
    let config = target_split[1];

    // defconfig contains "/.../nuttx/boards/risc-v/bl808/ox64/configs/nsh/defconfig"
    // Search for "/{board}/configs/{config}/defconfig"
    let search = format!("/{board}/configs/{config}/defconfig");
    let input = File::open(defconfig).unwrap();
    let buffered = BufReader::new(input);
    for line in buffered.lines() {
        let line = line.unwrap();
        if let Some(pos) = line.find(&search) {
            let s = &line[0..pos];
            let slash = s.rfind("/").unwrap();
            let subarch = s[slash + 1..].to_string();
            return Ok(subarch);
        }
    }
    Ok("unknown".into())
}
