use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: String,
    #[serde(default)]
    pub labels: Vec<LabelRef>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: String,
    #[serde(default)]
    pub labels: Vec<LabelRef>,
    pub created_at: String,
    pub merged_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LabelRef {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Secret {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Comment {
    pub body: String,
    pub created_at: String,
    pub author: Option<CommentAuthor>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CommentAuthor {
    pub login: String,
}

/// A timeline event from the GitHub API.
/// We only care about labeled/unlabeled events.
#[derive(Debug, Clone, Deserialize)]
pub struct TimelineEvent {
    pub event: Option<String>,
    pub label: Option<TimelineLabel>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimelineLabel {
    pub name: String,
}

pub trait GitHost {
    fn create_label(&self, name: &str, color: &str, description: &str) -> Result<()>;
    fn list_secrets(&self) -> Result<Vec<Secret>>;
    fn list_issues(&self, label: &str) -> Result<Vec<Issue>>;
    fn list_issues_any_label(&self, labels: &[&str]) -> Result<Vec<Issue>>;
    fn list_prs(&self, label: &str) -> Result<Vec<PullRequest>>;
    fn list_prs_any_label(&self, labels: &[&str]) -> Result<Vec<PullRequest>>;
    fn get_issue(&self, number: u64) -> Result<Issue>;
    fn get_issue_comments(&self, number: u64) -> Result<Vec<Comment>>;
    fn get_issue_timeline(&self, number: u64) -> Result<Vec<TimelineEvent>>;
    fn get_pr_comments(&self, number: u64) -> Result<Vec<Comment>>;
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

const ISSUE_FIELDS: &str = "number,title,url,state,labels,createdAt";
const PR_FIELDS: &str = "number,title,url,state,labels,createdAt,mergedAt";

impl GitHost for GhCli {
    fn create_label(&self, name: &str, color: &str, description: &str) -> Result<()> {
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
            "50",
            "--json",
            ISSUE_FIELDS,
        ])?;
        Ok(serde_json::from_slice(&output)?)
    }

    fn list_issues_any_label(&self, labels: &[&str]) -> Result<Vec<Issue>> {
        let mut all = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for label in labels {
            if let Ok(issues) = self.list_issues(label) {
                for issue in issues {
                    if seen.insert(issue.number) {
                        all.push(issue);
                    }
                }
            }
        }
        Ok(all)
    }

    fn list_prs(&self, label: &str) -> Result<Vec<PullRequest>> {
        let output = Self::run(&[
            "pr", "list", "--label", label, "--limit", "50", "--state", "all", "--json", PR_FIELDS,
        ])?;
        Ok(serde_json::from_slice(&output)?)
    }

    fn list_prs_any_label(&self, labels: &[&str]) -> Result<Vec<PullRequest>> {
        let mut all = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for label in labels {
            if let Ok(prs) = self.list_prs(label) {
                for pr in prs {
                    if seen.insert(pr.number) {
                        all.push(pr);
                    }
                }
            }
        }
        Ok(all)
    }

    fn get_issue(&self, number: u64) -> Result<Issue> {
        let num = number.to_string();
        let output = Self::run(&["issue", "view", &num, "--json", ISSUE_FIELDS])?;
        Ok(serde_json::from_slice(&output)?)
    }

    fn get_issue_comments(&self, number: u64) -> Result<Vec<Comment>> {
        let num = number.to_string();
        let output = Self::run(&["issue", "view", &num, "--json", "comments"])?;
        // gh returns {"comments": [...]}
        let wrapper: CommentWrapper = serde_json::from_slice(&output)?;
        Ok(wrapper.comments)
    }

    fn get_issue_timeline(&self, number: u64) -> Result<Vec<TimelineEvent>> {
        // Timeline requires the REST API directly
        let path = format!("repos/{{owner}}/{{repo}}/issues/{number}/timeline?per_page=100");
        let output = Self::run(&["api", &path, "--paginate"])?;
        Ok(serde_json::from_slice(&output)?)
    }

    fn get_pr_comments(&self, number: u64) -> Result<Vec<Comment>> {
        let num = number.to_string();
        let output = Self::run(&["pr", "view", &num, "--json", "comments"])?;
        let wrapper: CommentWrapper = serde_json::from_slice(&output)?;
        Ok(wrapper.comments)
    }
}

#[derive(Deserialize)]
struct CommentWrapper {
    comments: Vec<Comment>,
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
        pub comments: Vec<Comment>,
        pub timeline: Vec<TimelineEvent>,
    }

    impl FakeGitHost {
        pub fn new() -> Self {
            Self {
                labels_created: RefCell::new(Vec::new()),
                secrets: Vec::new(),
                issues: Vec::new(),
                prs: Vec::new(),
                comments: Vec::new(),
                timeline: Vec::new(),
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

        fn list_issues(&self, label: &str) -> Result<Vec<Issue>> {
            Ok(self
                .issues
                .iter()
                .filter(|i| i.labels.iter().any(|l| l.name == label))
                .cloned()
                .collect())
        }

        fn list_issues_any_label(&self, labels: &[&str]) -> Result<Vec<Issue>> {
            Ok(self
                .issues
                .iter()
                .filter(|i| i.labels.iter().any(|l| labels.contains(&l.name.as_str())))
                .cloned()
                .collect())
        }

        fn list_prs(&self, label: &str) -> Result<Vec<PullRequest>> {
            Ok(self
                .prs
                .iter()
                .filter(|p| p.labels.iter().any(|l| l.name == label))
                .cloned()
                .collect())
        }

        fn list_prs_any_label(&self, labels: &[&str]) -> Result<Vec<PullRequest>> {
            Ok(self
                .prs
                .iter()
                .filter(|p| p.labels.iter().any(|l| labels.contains(&l.name.as_str())))
                .cloned()
                .collect())
        }

        fn get_issue(&self, number: u64) -> Result<Issue> {
            self.issues
                .iter()
                .find(|i| i.number == number)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("issue #{number} not found"))
        }

        fn get_issue_comments(&self, _number: u64) -> Result<Vec<Comment>> {
            Ok(self.comments.clone())
        }

        fn get_issue_timeline(&self, _number: u64) -> Result<Vec<TimelineEvent>> {
            Ok(self.timeline.clone())
        }

        fn get_pr_comments(&self, _number: u64) -> Result<Vec<Comment>> {
            Ok(self.comments.clone())
        }
    }
}
