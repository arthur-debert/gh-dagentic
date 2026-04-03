use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Secret {
    pub name: String,
}

pub trait GitHost {
    fn create_label(&self, name: &str, color: &str, description: &str) -> Result<()>;
    fn list_secrets(&self) -> Result<Vec<Secret>>;
    fn list_issues(&self, label: &str) -> Result<Vec<Issue>>;
    fn list_prs(&self, label: &str) -> Result<Vec<PullRequest>>;
}

pub struct GhCli;

impl GhCli {
    fn run(args: &[&str]) -> Result<Vec<u8>> {
        let output = Command::new("gh")
            .args(args)
            .output()
            .context("failed to run gh CLI — is it installed?")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("gh {} failed: {}", args.join(" "), stderr.trim());
        }
        Ok(output.stdout)
    }
}

impl GitHost for GhCli {
    fn create_label(&self, name: &str, color: &str, description: &str) -> Result<()> {
        // --force makes this idempotent (updates if exists)
        let _ = Self::run(&[
            "label",
            "create",
            name,
            "--color",
            color,
            "--description",
            description,
            "--force",
        ]);
        Ok(())
    }

    fn list_secrets(&self) -> Result<Vec<Secret>> {
        let output = Self::run(&["secret", "list", "--json", "name"])?;
        Ok(serde_json::from_slice(&output)?)
    }

    fn list_issues(&self, label: &str) -> Result<Vec<Issue>> {
        let output = Self::run(&[
            "issue",
            "list",
            "--label",
            label,
            "--limit",
            "20",
            "--json",
            "number,title,url",
        ])?;
        Ok(serde_json::from_slice(&output)?)
    }

    fn list_prs(&self, label: &str) -> Result<Vec<PullRequest>> {
        let output = Self::run(&[
            "pr",
            "list",
            "--label",
            label,
            "--limit",
            "20",
            "--json",
            "number,title,url",
        ])?;
        Ok(serde_json::from_slice(&output)?)
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::cell::RefCell;

    pub struct FakeGitHost {
        pub labels_created: RefCell<Vec<(String, String, String)>>,
        pub secrets: Vec<Secret>,
        pub issues: Vec<Issue>,
        pub prs: Vec<PullRequest>,
    }

    impl FakeGitHost {
        pub fn new() -> Self {
            Self {
                labels_created: RefCell::new(Vec::new()),
                secrets: Vec::new(),
                issues: Vec::new(),
                prs: Vec::new(),
            }
        }

        pub fn with_secret(mut self, name: &str) -> Self {
            self.secrets.push(Secret {
                name: name.to_string(),
            });
            self
        }
    }

    impl GitHost for FakeGitHost {
        fn create_label(&self, name: &str, color: &str, description: &str) -> Result<()> {
            self.labels_created.borrow_mut().push((
                name.to_string(),
                color.to_string(),
                description.to_string(),
            ));
            Ok(())
        }

        fn list_secrets(&self) -> Result<Vec<Secret>> {
            Ok(self.secrets.clone())
        }

        fn list_issues(&self, _label: &str) -> Result<Vec<Issue>> {
            Ok(self.issues.clone())
        }

        fn list_prs(&self, _label: &str) -> Result<Vec<PullRequest>> {
            Ok(self.prs.clone())
        }
    }
}
