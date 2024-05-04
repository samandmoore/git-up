mod git;

use std::{collections::HashMap, io::BufRead, process::Command};

use colored::*;
use log::{debug, info};

use anyhow::{Context, Result};

fn main() -> Result<()> {
    env_logger::init();

    // 1. get main remote
    let remote = "origin";

    // 2. get default branch
    let mut current_branch = String::new();

    // 3. get current branch
    let default_branch = "main";
    let full_default_branch = format!("refs/remotes/{}/{}", remote, default_branch);

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
        .with_context(|| "Failed to execute git fetch command")?
        .wait()?;

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

    let branches_to_remotes: HashMap<String, String> = output
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
    debug!("Map of branches to remotes: {:?}", branches_to_remotes);

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

    // 7. loop over branches, updating local if remote has changed, delete if merged, warn if not merged
    for local_branch in local_branches {
        let result = process_branch(
            &local_branch,
            remote,
            &current_branch,
            &branches_to_remotes,
            &default_branch,
            &full_default_branch,
        );
        match result {
            Ok(Some(branch)) => {
                current_branch = branch;
            }
            Ok(None) => {}
            Err(e) => {
                println!(
                    "{} {}{} failed to process branch: {}",
                    "Error:".red(),
                    local_branch.red().bold(),
                    "".clear(),
                    e
                );
            }
        }
    }

    Ok(())
}

fn process_branch(
    local_branch: &str,
    remote: &str,
    current_branch: &str,
    branches_to_remotes: &HashMap<String, String>,
    default_branch: &str,
    full_default_branch: &str,
) -> Result<Option<String>> {
    let full_branch = format!("refs/heads/{}", local_branch);
    let mut remote_branch = format!("refs/remotes/{}/{}", remote, local_branch);
    let mut gone = false;

    info!("Checking branch {}", local_branch);
    if let Some(local_branch_remote_name) = branches_to_remotes.get(local_branch) {
        if local_branch_remote_name == remote {
            if let Some(symbolic_full_name) =
                git::symbolic_full_name(format!("{}@{{upstream}}", local_branch))
            {
                debug!("Symbolic full name is {}", symbolic_full_name);
                remote_branch = symbolic_full_name;
            } else {
                debug!("No symbolic full name found for {}", local_branch);
                remote_branch = String::new();
                gone = true;
            }
            debug!("Remote is {}", local_branch_remote_name);
        } else if !git::has_file(&remote_branch) {
            remote_branch = String::new();
        }
    }

    if !remote_branch.is_empty() {
        let diff = git::make_range(&full_branch, &remote_branch)?;

        if diff.is_identical() {
            return Ok(None);
        } else if diff.is_ancestor() {
            if local_branch == current_branch {
                match git::fast_forward_merge(&remote_branch) {
                    Err(e) => {
                        println!(
                            "{} {}{} failed to fast forward merge: {}",
                            "Error:".red(),
                            local_branch.red().bold(),
                            "".clear(),
                            e
                        );
                    }
                    _ => {}
                }
            } else {
                match git::update_ref(&full_branch, &remote_branch) {
                    Err(e) => {
                        println!(
                            "{} {}{} failed to update ref: {}",
                            "Error:".red(),
                            local_branch.red().bold(),
                            "".clear(),
                            e
                        );
                    }
                    _ => {}
                }
            }
            println!(
                "{} {}{} (was {}).",
                "Updated branch".green(),
                local_branch.green().bold(),
                "".clear(),
                diff.a[0..7].to_string(),
            );
            Ok(None)
        } else {
            println!(
                "{} {}{} seems to contain unpushed commits",
                "Warning:".yellow(),
                local_branch.yellow().bold(),
                "".clear()
            );
            Ok(None)
        }
    } else if gone {
        let diff = git::make_range(&full_branch, &full_default_branch)?;
        if diff.is_ancestor() {
            if local_branch == current_branch {
                git::checkout(default_branch)?;
            }
            git::delete_branch(&local_branch)?;
            println!(
                "{} {}{} (was {}).",
                "Deleted branch".red(),
                local_branch.red().bold(),
                "".clear(),
                diff.a[0..7].to_string(),
            );
            if local_branch == current_branch {
                return Ok(Some(String::from(default_branch)));
            } else {
                return Ok(None);
            }
        }
        Ok(None)
    } else {
        println!(
            "{} '{}'{} was deleted on {}, but appears not merged into '{}'",
            "Warning:".yellow(),
            local_branch.yellow().bold(),
            "".clear(),
            remote,
            current_branch,
        );
        Ok(None)
    }
}