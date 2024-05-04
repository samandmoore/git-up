use std::{collections::HashMap, io::BufRead, path::Path, process::Command};

use colored::*;
use log::{debug, info};

use anyhow::{anyhow, Context, Result};

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
                git_symbolic_full_name(format!("{}@{{upstream}}", local_branch))
            {
                debug!("Symbolic full name is {}", symbolic_full_name);
                remote_branch = symbolic_full_name;
            } else {
                debug!("No symbolic full name found for {}", local_branch);
                remote_branch = String::new();
                gone = true;
            }
            debug!("Remote is {}", local_branch_remote_name);
        } else if !git_has_file(&remote_branch) {
            remote_branch = String::new();
        }
    }

    if !remote_branch.is_empty() {
        let diff = git_range(&full_branch, &remote_branch)?;

        if diff.is_identical() {
            return Ok(None);
        } else if diff.is_ancestor() {
            if local_branch == current_branch {
                match git_fast_forward_merge(&remote_branch) {
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
                match git_update_ref(&full_branch, &remote_branch) {
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
        let diff = git_range(&full_branch, &full_default_branch)?;
        if diff.is_ancestor() {
            if local_branch == current_branch {
                git_checkout(default_branch)?;
            }
            git_delete_branch(&local_branch)?;
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

fn git_delete_branch(local_branch: &str) -> Result<()> {
    let result = Command::new("git")
        .arg("branch")
        .arg("-D")
        .arg("--quiet")
        .arg(local_branch)
        .output()?;

    if result.status.success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to delete branch"))
    }
}

fn git_checkout(branch: &str) -> Result<()> {
    let result = Command::new("git")
        .arg("checkout")
        .arg("--quiet")
        .arg(branch)
        .output()?;

    if result.status.success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to checkout branch"))
    }
}

fn git_update_ref(full_branch: &str, remote_branch: &str) -> Result<()> {
    let result = Command::new("git")
        .arg("update-ref")
        .arg(full_branch)
        .arg(remote_branch)
        .output()?;

    if result.status.success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to update ref"))
    }
}

fn git_fast_forward_merge(branch: &str) -> Result<()> {
    let result = Command::new("git")
        .arg("merge")
        .arg("--ff-only")
        .arg("--quiet")
        .arg(branch)
        .output()?;

    if result.status.success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to fast forward merge ref"))
    }
}

struct Range {
    a: String,
    b: String,
}

impl Range {
    fn new(a: String, b: String) -> Self {
        Self { a, b }
    }

    fn is_identical(&self) -> bool {
        self.a == self.b
    }

    fn is_ancestor(&self) -> bool {
        git_is_ancestor(&self.a, &self.b)
    }
}

fn git_is_ancestor(a: &str, b: &str) -> bool {
    let result = Command::new("git")
        .arg("merge-base")
        .arg("--is-ancestor")
        .arg(a)
        .arg(b)
        .output();

    match result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn git_range(a: &str, b: &str) -> Result<Range> {
    let result = Command::new("git")
        .arg("rev-parse")
        .arg("--quiet")
        .arg(a)
        .arg(b)
        .output()?;

    let lines = output_lines(result);
    if lines.len() != 2 {
        return Err(anyhow!(
            "Can't parse range {}..{}; Expected 2 lines, got {}",
            a,
            b,
            lines.len()
        ));
    }

    Ok(Range::new(lines[0].clone(), lines[1].clone()))
}

fn output_lines(output: std::process::Output) -> Vec<String> {
    output.stdout.lines().map(|line| line.unwrap()).collect()
}

fn git_has_file(path: &str) -> bool {
    let result = Command::new("git")
        .arg("rev-parse")
        .arg("--quiet")
        .arg("--git-path")
        .arg(path)
        .output();

    match result {
        Ok(output) => {
            if output.status.success() {
                let file_path = String::from_utf8(output.stdout).unwrap();
                Path::new(file_path.trim()).exists()
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

fn git_symbolic_full_name(name: String) -> Option<String> {
    let result = Command::new("git")
        .arg("rev-parse")
        .arg("--symbolic-full-name")
        .arg(name)
        .output();

    match result {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8(output.stdout).unwrap();
                Some(stdout.trim().to_string())
            } else {
                None
            }
        }
        Err(_) => None,
    }
}
