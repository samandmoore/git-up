mod git;

use std::{collections::HashMap, io::BufRead, process::Command};

use colored::*;
use log::{debug, info};

use anyhow::{Context, Result};
use tap::Tap;

fn main() -> Result<()> {
    env_logger::init();

    let remote = git::get_main_remote()?;

    let default_branch = git::get_default_branch(&remote)?;

    let full_default_branch = format!("refs/remotes/{}/{}", remote, default_branch);

    let mut current_branch =
        git::symbolic_ref("HEAD", true).with_context(|| "Failed to get current branch")?;

    // 4. fetch from remote `git fetch --prune --quiet --progress $remote`
    Command::new("git")
        .arg("fetch")
        .arg("--prune")
        .arg("--quiet")
        .arg("--progress")
        .arg(&remote)
        .tap(|command| {
            info!("Fetching from remote");
            debug!("Fetching from remote with command {:?}", command);
        })
        .spawn()
        .with_context(|| "Failed to execute git fetch command")?
        .wait()?;

    // 5. gather list of branch -> remote mappings `git config --local --get-regexp branch.*.remote`
    let output = Command::new("git")
        .arg("config")
        .arg("--local")
        .arg("--get-regexp")
        .arg("branch.*.remote")
        .tap(|command| {
            info!("Getting branch -> remote mappings");
            debug!(
                "Getting branch -> remote mappings with command {:?}",
                command
            );
        })
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
    let output = Command::new("git")
        .arg("branch")
        .arg("--list")
        .tap(|command| {
            info!("Getting local branches");
            debug!("Getting local branches with command {:?}", command);
        })
        .output()
        .with_context(|| "Failed to execute git branch command")?;

    let local_branches: Vec<String> = output
        .stdout
        .lines()
        .map(|line| String::from(line.unwrap().trim().split(' ').last().unwrap()))
        .collect();

    // 7. loop over branches, updating local if remote has changed, delete if merged, warn if not merged
    for local_branch in local_branches {
        let result = process_branch(
            &local_branch,
            &remote,
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

enum BranchStatus {
    RemoteBranchExists(String),
    RemoteBranchGone,
    RemoteBranchPotentiallyUnmerged,
}

fn determine_branch_status(
    local_branch: &str,
    remote: &str,
    branches_to_remotes: &HashMap<String, String>,
) -> BranchStatus {
    let remote_branch = format!("refs/remotes/{}/{}", remote, local_branch);

    if let Some(local_branch_remote_name) = branches_to_remotes.get(local_branch) {
        if local_branch_remote_name == remote {
            if let Some(symbolic_full_name) =
                git::symbolic_full_name(format!("{}@{{upstream}}", local_branch))
            {
                debug!("Symbolic full name is {}", symbolic_full_name);
                BranchStatus::RemoteBranchExists(symbolic_full_name)
            } else {
                debug!("No symbolic full name found for {}", local_branch);
                BranchStatus::RemoteBranchGone
            }
        } else if !git::has_file(&remote_branch) {
            BranchStatus::RemoteBranchPotentiallyUnmerged
        } else {
            BranchStatus::RemoteBranchExists(remote_branch.clone())
        }
    } else {
        BranchStatus::RemoteBranchExists(remote_branch.clone())
    }
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

    info!("Checking branch {}", local_branch);
    let branch_status = determine_branch_status(local_branch, remote, branches_to_remotes);

    match branch_status {
        BranchStatus::RemoteBranchExists(remote_branch) => {
            let diff = git::make_range(&full_branch, &remote_branch)?;

            if diff.is_identical() {
                return Ok(None);
            } else if diff.is_ancestor() {
                if local_branch == current_branch {
                    git::fast_forward_merge(&remote_branch)
                        .with_context(|| "failed to fast forward merge")?;
                } else {
                    git::update_ref(&full_branch, &remote_branch)
                        .with_context(|| "failed to update ref")?;
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
        }
        BranchStatus::RemoteBranchGone => {
            let diff = git::make_range(&full_branch, &full_default_branch)?;
            if diff.is_ancestor() {
                if local_branch == current_branch {
                    git::checkout(default_branch)
                        .with_context(|| "failed to checkout default branch")?;
                }
                git::delete_branch(&local_branch)
                    .with_context(|| "failed to delete local branch")?;
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
        }
        BranchStatus::RemoteBranchPotentiallyUnmerged => {
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
}
