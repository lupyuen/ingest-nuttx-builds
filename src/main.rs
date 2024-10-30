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
        let id = gist.id;
        let url = gist.html_url;
        let file = gist.files.first_entry().unwrap();
        let raw_url = file.get().raw_url.as_str();
        let description = gist
            .description
            .unwrap_or("<No description>".into());
        println!("{id} | {url} | {description}");
        println!("raw_url={raw_url:?}");

        // Download the Gist
        let res = reqwest::get(raw_url).await?;
        println!("Status: {}", res.status());
        println!("Headers:\n{:#?}", res.headers());    
        let body = res.text().await?;
        println!("Body:\n{}", body);

        // Process the Build Log
        process_log(&body).await?;

        // Wait a while
        sleep(Duration::from_secs(5));
    }

    // Return OK
    Ok(())
}

/// Process the Build Log:
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
    Ok(())
}
