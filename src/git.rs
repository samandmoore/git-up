use anyhow::{anyhow, Context, Result};
use log::debug;
use std::{
    io::BufRead,
    path::Path,
    process::{Command, Output},
};
use tap::{Tap, TapFallible};

pub fn delete_branch(local_branch: &str) -> Result<()> {
    let result = Command::new("git")
        .arg("branch")
        .arg("-D")
        .arg("--quiet")
        .arg(local_branch)
        .run_for_output()?;

    if result.status.success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to delete branch"))
    }
}

pub fn checkout(branch: &str) -> Result<()> {
    let result = Command::new("git")
        .arg("checkout")
        .arg("--quiet")
        .arg(branch)
        .run_for_output()?;

    if result.status.success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to checkout branch"))
    }
}

pub fn update_ref(full_branch: &str, remote_branch: &str) -> Result<()> {
    let result = Command::new("git")
        .arg("update-ref")
        .arg(full_branch)
        .arg(remote_branch)
        .run_for_output()?;

    if result.status.success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to update ref"))
    }
}

pub fn fast_forward_merge(branch: &str) -> Result<()> {
    let result = Command::new("git")
        .arg("merge")
        .arg("--ff-only")
        .arg("--quiet")
        .arg(branch)
        .run_for_output()?;

    if result.status.success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to fast forward merge ref"))
    }
}

pub struct Range {
    pub a: String,
    pub b: String,
}

impl Range {
    pub fn new(a: String, b: String) -> Self {
        Self { a, b }
    }

    pub fn is_identical(&self) -> bool {
        self.a == self.b
    }

    pub fn is_ancestor(&self) -> bool {
        is_ancestor(&self.a, &self.b)
    }
}

fn is_ancestor(a: &str, b: &str) -> bool {
    let result = Command::new("git")
        .arg("merge-base")
        .arg("--is-ancestor")
        .arg(a)
        .arg(b)
        .run_for_output();

    match result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

pub fn make_range(a: &str, b: &str) -> Result<Range> {
    let result = Command::new("git")
        .arg("rev-parse")
        .arg("--quiet")
        .arg(a)
        .arg(b)
        .run_for_output()?;

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

pub fn has_file(path: &str) -> bool {
    let result = Command::new("git")
        .arg("rev-parse")
        .arg("--quiet")
        .arg("--git-path")
        .arg(path)
        .run_for_output();

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

pub fn symbolic_full_name(name: String) -> Option<String> {
    let result = Command::new("git")
        .arg("rev-parse")
        .arg("--symbolic-full-name")
        .arg(name)
        .run_for_output();

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

pub fn symbolic_ref(name: &str, short: bool) -> Option<String> {
    let result = Command::new("git")
        .arg("symbolic-ref")
        .arg("--quiet")
        .tap_mut(|command| {
            if short {
                command.arg("--short");
            }
        })
        .arg(name)
        .run_for_output();

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

pub fn get_main_remote() -> Result<String> {
    let result = Command::new("git")
        .arg("remote")
        .arg("--verbose")
        .run_for_output()?;

    if result.status.success() {
        // $ git remote --verbose
        // origin  git@github.com:samandmoore/git-up.git (fetch)
        // origin  git@github.com:samandmoore/git-up.git (push)
        let lines = output_lines(result);
        if lines.len() > 0 {
            return Ok(lines[0].split_whitespace().next().unwrap().to_string());
        }
        return Err(anyhow!("No remotes found"));
    } else {
        return Err(anyhow!("No remotes found"));
    }
}

pub fn get_default_branch(remote: &str) -> Result<String> {
    // the ref/remotes/X/HEAD ref will always be missing if you didn't `git clone` the repository
    symbolic_ref(&format!("refs/remotes/{}/HEAD", remote), false)
        // if it is missing, we assume "main"
        .unwrap_or(format!("refs/remotes/{}/main", remote))
        .strip_prefix(&format!("refs/remotes/{}/", remote))
        .map(|s| s.to_string())
        .ok_or(anyhow!("Failed to get default branch"))
}

pub fn fetch(remote: &str) -> Result<()> {
    Command::new("git")
        .arg("fetch")
        .arg("--prune")
        .arg("--quiet")
        .arg("--progress")
        .arg(remote)
        .run()
        .with_context(|| "Failed to execute git fetch command")
}

pub fn get_config(args: &[&str]) -> Result<Vec<String>> {
    let result = Command::new("git")
        .arg("config")
        .args(args)
        .run_for_output()?;

    if result.status.success() {
        Ok(output_lines(result))
    } else {
        Err(anyhow!("Failed to get config"))
    }
}

pub fn get_branches() -> Result<Vec<String>> {
    let result = Command::new("git")
        .arg("branch")
        .arg("--list")
        .arg("--format")
        .arg("%(refname:short)")
        .run_for_output()?;

    if result.status.success() {
        Ok(output_lines(result))
    } else {
        Err(anyhow!("Failed to get branches"))
    }
}

trait Runnable {
    fn run_for_output(&mut self) -> Result<Output>;
    fn run(&mut self) -> Result<()>;
}

impl Runnable for Command {
    fn run_for_output(&mut self) -> Result<Output> {
        debug!("Running command: {:?}", self);
        self.output()
            .tap_ok(|output| {
                debug!("Command result: {:?}", output);
            })
            .with_context(|| "Failed to execute command")
    }

    fn run(&mut self) -> Result<()> {
        debug!("Running command: {:?}", self);
        let result = self
            .spawn()?
            .wait()
            .with_context(|| "Failed to execute command")?;

        if result.success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to execute command"))
        }
    }
}
