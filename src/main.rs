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

    git::fetch(&remote).with_context(|| "Failed to execute git fetch command")?;

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

    for local_branch in local_branches {
        let current_branch =
            git::symbolic_ref("HEAD", true).with_context(|| "Failed to get current branch")?;
        let sync_context = SyncContext {
            remote: remote.clone(),
            default_branch: default_branch.clone(),
            full_default_branch: full_default_branch.clone(),
            local_branch: local_branch.clone(),
            current_branch: current_branch,
            branches_to_remotes: branches_to_remotes.clone(),
        };
        let result = process_branch(&sync_context);
        match result {
            Ok(_) => {}
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

struct SyncContext {
    remote: String,
    default_branch: String,
    full_default_branch: String,
    local_branch: String,
    current_branch: String,
    branches_to_remotes: HashMap<String, String>,
}

enum BranchStatus {
    RemoteBranchExists(String),
    RemoteBranchGone,
    Unknown,
}

impl SyncContext {
    fn determine_branch_status(&self) -> BranchStatus {
        let SyncContext {
            remote,
            local_branch,
            branches_to_remotes,
            ..
        } = self;
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
                BranchStatus::Unknown
            } else {
                BranchStatus::RemoteBranchExists(remote_branch.clone())
            }
        } else {
            BranchStatus::RemoteBranchExists(remote_branch.clone())
        }
    }
}

fn process_branch(sync_context: &SyncContext) -> Result<()> {
    let SyncContext {
        remote,
        default_branch,
        full_default_branch,
        local_branch,
        current_branch,
        ..
    } = sync_context;
    let full_branch = format!("refs/heads/{}", local_branch);

    info!("Checking branch {}", local_branch);
    let branch_status = sync_context.determine_branch_status();

    match branch_status {
        BranchStatus::RemoteBranchExists(remote_branch) => {
            let range = git::make_range(&full_branch, &remote_branch)?;

            if range.is_identical() {
                return Ok(());
            } else if range.is_ancestor() {
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
                    range.a[0..7].to_string(),
                );
                Ok(())
            } else {
                println!(
                    "{} {}{} seems to contain unpushed commits",
                    "Warning:".yellow(),
                    local_branch.yellow().bold(),
                    "".clear()
                );
                Ok(())
            }
        }
        BranchStatus::RemoteBranchGone => {
            let range = git::make_range(&full_branch, &full_default_branch)?;
            if range.is_ancestor() {
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
                    range.a[0..7].to_string(),
                );
            } else {
                println!(
                    "{} {}{} was deleted on {}, but appears not merged into {}",
                    "Warning:".yellow(),
                    local_branch.yellow().bold(),
                    "".clear(),
                    remote,
                    default_branch.bold(),
                );
            }
            Ok(())
        }
        BranchStatus::Unknown => Ok(()),
    }
}
