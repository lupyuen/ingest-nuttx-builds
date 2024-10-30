//! Fetch the Latest 20 PRs:
//!   If PR Status = Open
//!   And PR Comments don't exist:
//!     Then Call Gemini API to Validate the PR
//!     And Post Gemini Response as PR Comment

use std::{
    thread::sleep, 
    time::Duration,
    vec,
};
use clap::Parser;
use regex::Regex;

/// Command-Line Arguments
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Owner of the GitHub Gists that will be processed (`nuttx`)
    #[arg(long)]
    user: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Init the Logger and Command-Line Args
    env_logger::init();
    let args = Args::parse();

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
        .per_page(20)
        .send()
        .await?;

    // Process Every Gist
    println!("id | url | description");
    for mut gist in gists {
        let id = gist.id;  // "6e5150f02e081be935fa525e6546cb2b"
        let url = gist.html_url;  // "https://gist.github.com/nuttxpr/6e5150f02e081be935fa525e6546cb2b"
        let file = gist.files.first_entry().unwrap();
        let filename = file.get().filename.as_str();  // "ci-arm-04.log"
        let raw_url = file.get().raw_url.as_str();  // "https://gist.githubusercontent.com/nuttxpr/6e5150f02e081be935fa525e6546cb2b/raw/9f07185404c0f81914f622c0152a980022539968/ci-arm-04.log"
        let description = gist
            .description
            .unwrap_or("<No description>".into());  // "[arm-04] CI Log for nuttx @ f6facf7602003071aaabc6dd00082b7ebb2f5ab9 / nuttx-apps @ d9e178aad022030224d1c95628cab1784a13a339"
        let target_group = filename
            .replace("ci-", "")
            .replace(".log", "");  // "arm-04"
        println!("{id} | {url} | {description}");
        println!("target_group={target_group:?}");
        println!("filename={filename:?}");
        println!("raw_url={raw_url:?}");

        // TODO: Skip the filenames we've seen before

        // TODO: Compose the Line URL
        // https://gist.github.com/nuttxpr/6e5150f02e081be935fa525e6546cb2b#file-ci-arm-04-log-L140

        // Download the Gist
        let res = reqwest::get(raw_url).await?;
        // println!("Status: {}", res.status());
        // println!("Headers:\n{:#?}", res.headers());    
        let body = res.text().await?;
        // println!("Body:\n{}", body);

        // Process the Build Log
        process_log(&body).await?;

        // Wait a while
        sleep(Duration::from_secs(5));
    }

    // Return OK
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
async fn process_log(log: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Look for the delimiter
    const DELIMITER: &str = "==========";
    let mut target_linenum: Option<usize> = None;
    let lines = &log.split('\n').collect::<Vec<_>>();
    for (linenum, line) in lines.into_iter().enumerate() {
        if line.starts_with(DELIMITER) {
            // Process the target
            if let Some(l) = target_linenum {
                let target = &lines[l..linenum];
                process_target(target, l).await?;
            }
            target_linenum = Some(linenum + 1);
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
async fn process_target(lines: &[&str], linenum: usize) -> Result<(), Box<dyn std::error::Error>> {
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
    let timestamp = lines[l]
        .replace(" ", "T");
    println!("timestamp={timestamp}");
    l += 1;

    // To Identify Errors / Warnings: Skip the known lines
    let mut msg: Vec<&str> = vec![];
    let lines = &lines[l..];
    for line in lines {
        let line = line.trim();
        if line.starts_with("----------") ||
            line.starts_with("Cleaning") ||
            line.starts_with("Configuring") ||
            line.starts_with("Select") ||
            line.starts_with("Disabling") ||
            line.starts_with("Enabling") ||
            line.starts_with("Building") ||
            line.starts_with("Normalize") {
                continue;
            }
            println!("*** Error / Warning: {line}");
            msg.push(line);
    }

    // Post the Target to Prometheus Pushgateway
    let version = 1;

    sleep(Duration::from_secs(5));
    Ok(())
}
