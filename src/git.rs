use anyhow::{Result, anyhow};
use cmd_proc::Command;
use log::debug;
use std::path::Path;

/// Helper to create a git base command
fn git_cmd() -> Command {
    Command::new("git")
}

pub fn delete_branch(local_branch: &str) -> Result<()> {
    debug!("Running: git branch -D --quiet {}", local_branch);

    git_cmd()
        .argument("branch")
        .argument("-D")
        .argument("--quiet")
        .argument(local_branch)
        .status()
        .map_err(|e| anyhow!("Failed to delete branch: {:?}", e))
}

pub fn checkout(branch: &str) -> Result<()> {
    debug!("Running: git checkout --quiet {}", branch);

    git_cmd()
        .argument("checkout")
        .argument("--quiet")
        .argument(branch)
        .status()
        .map_err(|e| anyhow!("Failed to checkout branch: {:?}", e))
}

pub fn update_ref(full_branch: &str, remote_branch: &str) -> Result<()> {
    debug!("Running: git update-ref {} {}", full_branch, remote_branch);

    git_cmd()
        .argument("update-ref")
        .argument(full_branch)
        .argument(remote_branch)
        .status()
        .map_err(|e| anyhow!("Failed to update ref: {:?}", e))
}

pub fn fast_forward_merge(branch: &str) -> Result<()> {
    debug!("Running: git merge --ff-only --quiet {}", branch);

    git_cmd()
        .argument("merge")
        .argument("--ff-only")
        .argument("--quiet")
        .argument(branch)
        .status()
        .map_err(|e| anyhow!("Failed to fast forward merge ref: {:?}", e))
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
    debug!("Running: git merge-base --is-ancestor {} {}", a, b);

    git_cmd()
        .argument("merge-base")
        .argument("--is-ancestor")
        .argument(a)
        .argument(b)
        .status()
        .is_ok()
}

pub fn make_range(a: &str, b: &str) -> Result<Range> {
    debug!("Running: git rev-parse --quiet {} {}", a, b);

    let sha_a = git_proc::rev_parse::new()
        .rev(a)
        .stdout()
        .string()
        .map_err(|e| anyhow!("Failed to rev-parse {}: {:?}", a, e))?;

    let sha_b = git_proc::rev_parse::new()
        .rev(b)
        .stdout()
        .string()
        .map_err(|e| anyhow!("Failed to rev-parse {}: {:?}", b, e))?;

    Ok(Range::new(sha_a, sha_b))
}

pub fn has_file(path: &str) -> bool {
    debug!("Running: git rev-parse --quiet --git-path {}", path);

    let result = git_cmd()
        .argument("rev-parse")
        .argument("--quiet")
        .argument("--git-path")
        .argument(path)
        .stdout()
        .string();

    match result {
        Ok(output) => Path::new(output.trim()).exists(),
        Err(_) => false,
    }
}

pub fn symbolic_full_name(name: String) -> Option<String> {
    debug!("Running: git rev-parse --symbolic-full-name {}", name);

    git_proc::rev_parse::new()
        .symbolic_full_name()
        .rev(&name)
        .stdout()
        .string()
        .ok()
        .map(|s: String| s.trim().to_string())
}

pub fn symbolic_ref(name: &str, short: bool) -> Option<String> {
    debug!(
        "Running: git symbolic-ref --quiet {} {}",
        if short { "--short" } else { "" },
        name
    );

    git_cmd()
        .argument("symbolic-ref")
        .argument("--quiet")
        .optional_argument(short.then_some("--short"))
        .argument(name)
        .stdout()
        .string()
        .ok()
        .map(|s: String| s.trim().to_string())
}

pub fn get_main_remote() -> Result<String> {
    debug!("Running: git remote --verbose");

    let output = git_cmd()
        .argument("remote")
        .argument("--verbose")
        .stdout()
        .string()
        .map_err(|e| anyhow!("Failed to get remotes: {:?}", e))?;

    let lines: Vec<&str> = output.lines().collect();
    if !lines.is_empty() {
        return Ok(lines[0].split_whitespace().next().unwrap().to_string());
    }

    Err(anyhow!("No remotes found"))
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
    debug!("Running: git fetch --prune --quiet --progress {}", remote);

    git_cmd()
        .argument("fetch")
        .argument("--prune")
        .argument("--quiet")
        .argument("--progress")
        .argument(remote)
        .spawn()
        .run()
        .map_err(|e| anyhow!("Failed to execute git fetch command: {:?}", e))?
        .wait()
        .map_err(|e| anyhow!("Failed to wait for git fetch: {:?}", e))?;

    Ok(())
}

pub fn get_config(args: &[&str]) -> Result<Vec<String>> {
    debug!("Running: git config {:?}", args);

    let output = git_cmd()
        .argument("config")
        .arguments(args.iter())
        .stdout()
        .string()
        .map_err(|e| anyhow!("Failed to get config: {:?}", e))?;

    Ok(output.lines().map(|s: &str| s.to_string()).collect())
}

pub fn get_branches() -> Result<Vec<String>> {
    debug!("Running: git branch --list --format %(refname:short)");

    let output = git_cmd()
        .argument("branch")
        .argument("--list")
        .argument("--format")
        .argument("%(refname:short)")
        .stdout()
        .string()
        .map_err(|e| anyhow!("Failed to get branches: {:?}", e))?;

    Ok(output.lines().map(|s: &str| s.to_string()).collect())
}
