use std::{io::BufRead, process::Command};

use log::{debug, info};

use anyhow::{Context, Result};

fn main() -> Result<()> {
    env_logger::init();

    // 1. get main remote
    let remote = "origin";

    // 2. get default branch
    // 3. get current branch

    // 4. fetch from remote `git fetch --prune --quiet --progress $remote`
    let mut command = Command::new("git");
    let command = command
        .arg("fetch")
        .arg("--prune")
        .arg("--quiet")
        .arg("--progress")
        .arg(remote);

    info!("Fetching from remote");
    debug!("Fetching from remote with command {:?}", command);
    command
        .spawn()
        .with_context(|| "Failed to execute git fetch command")?;

    // 5. gather list of branch -> remote mappings `git config --local --get-regexp branch.*.remote`
    let mut command = Command::new("git");
    let command = command
        .arg("config")
        .arg("--local")
        .arg("--get-regexp")
        .arg("branch.*.remote");

    info!("Getting branch -> remote mappings");
    debug!(
        "Getting branch -> remote mappings with command {:?}",
        command
    );
    let output = command
        .output()
        .with_context(|| "Failed to execute git config command")?;

    let branches_to_remotes: Vec<(String, String)> = output
        .stdout
        .lines()
        .map(|line| {
            let parts: Vec<String> = line.unwrap().split(' ').map(String::from).collect();
            (
                parts[0].split('.').skip(1).take(1).collect(),
                parts[1].clone(),
            )
        })
        .collect();
    println!("{:?}", branches_to_remotes);

    // 6. gather list of local branches `git branch --list`
    let mut command = Command::new("git");
    let command = command.arg("branch").arg("--list");
    info!("Getting local branches");
    debug!("Getting local branches with command {:?}", command);
    let output = command
        .output()
        .with_context(|| "Failed to execute git config command")?;

    let local_branches: Vec<String> = output
        .stdout
        .lines()
        .map(|line| String::from(line.unwrap().trim().split(' ').last().unwrap()))
        .collect();
    println!("{:?}", local_branches);

    // 7. loop over branches, updating local if remote has changed, delete if merged, warn if not merged

    Ok(())
}
