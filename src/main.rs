//! Fetch the Latest 20 PRs:
//!   If PR Status = Open
//!   And PR Comments don't exist:
//!     Then Call Gemini API to Validate the PR
//!     And Post Gemini Response as PR Comment

use std::{
    env, 
    io::Read,
    thread::sleep, 
    time::Duration
};
use clap::Parser;
use env_logger::Target;
use log::info;

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

    // Fetch the Latest Gists
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
        let filename = file.get().filename.as_str();
        let raw_url = file.get().raw_url.as_str();
        let description = gist  // "[arm-04] CI Log for nuttx @ f6facf7602003071aaabc6dd00082b7ebb2f5ab9 / nuttx-apps @ d9e178aad022030224d1c95628cab1784a13a339"
            .description
            .unwrap_or("<No description>".into());
        println!("{id} | {url} | {description}");
        println!("filename={filename:?}");  // "ci-arm-04.log"
        println!("raw_url={raw_url:?}");

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
    const delimiter: &str = "==========";
    let mut target_linenum: Option<usize> = None;
    let lines = &log.split('\n').collect::<Vec<_>>();
    for (linenum, line) in lines.into_iter().enumerate() {
        if line.starts_with(delimiter) {
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

/// Process a Build Target:
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
async fn process_target(target: &[&str], linenum: usize) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", target[0]);
    sleep(Duration::from_secs(5));
    Ok(())
}
