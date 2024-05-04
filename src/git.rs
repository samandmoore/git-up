use anyhow::{anyhow, Result};
use std::{io::BufRead, path::Path, process::Command};

pub fn delete_branch(local_branch: &str) -> Result<()> {
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

pub fn checkout(branch: &str) -> Result<()> {
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

pub fn update_ref(full_branch: &str, remote_branch: &str) -> Result<()> {
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

pub fn fast_forward_merge(branch: &str) -> Result<()> {
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
        .output();

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

pub fn has_file(path: &str) -> bool {
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

pub fn symbolic_full_name(name: String) -> Option<String> {
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