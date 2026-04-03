use crate::context::Context;
use crate::git;
use anyhow::{Result, bail};

struct Section {
    title: &'static str,
    label: &'static str,
    kind: ItemKind,
}

enum ItemKind {
    Issue,
    Pr,
}

const SECTIONS: &[Section] = &[
    Section {
        title: "Issues awaiting planning",
        label: "status: needs-plan",
        kind: ItemKind::Issue,
    },
    Section {
        title: "Plans ready for review",
        label: "status: plan-ready",
        kind: ItemKind::Issue,
    },
    Section {
        title: "Approved for implementation",
        label: "status: plan-approved",
        kind: ItemKind::Issue,
    },
    Section {
        title: "PRs awaiting review",
        label: "pr: review-pending",
        kind: ItemKind::Pr,
    },
    Section {
        title: "PRs with review addressed",
        label: "pr: review-addressed",
        kind: ItemKind::Pr,
    },
];

pub fn run(ctx: &Context) -> Result<()> {
    let _root = ctx.repo.repo_root()?;
    let remote = ctx.repo.remote_url("origin")?;
    if !git::is_github_url(&remote) {
        bail!(
            "origin remote does not point to GitHub.\n  Found: {}",
            remote
        );
    }

    println!("\x1b[1mDagentic Pipeline Status\x1b[0m\n");

    for section in SECTIONS {
        println!("\x1b[1m{}\x1b[0m ({}):", section.title, section.label);
        match &section.kind {
            ItemKind::Issue => match ctx.host.list_issues(section.label) {
                Ok(issues) if issues.is_empty() => println!("  (none)"),
                Ok(issues) => {
                    for issue in issues {
                        println!("  #{} {}", issue.number, issue.title);
                    }
                }
                Err(_) => println!("  (none)"),
            },
            ItemKind::Pr => match ctx.host.list_prs(section.label) {
                Ok(prs) if prs.is_empty() => println!("  (none)"),
                Ok(prs) => {
                    for pr in prs {
                        println!("  #{} {}", pr.number, pr.title);
                    }
                }
                Err(_) => println!("  (none)"),
            },
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use crate::fs::fake::FakeFs;
    use crate::gh::Issue;
    use crate::gh::fake::FakeGitHost;
    use crate::git::fake::FakeGitRepo;
    use std::path::PathBuf;

    #[test]
    fn status_runs_with_empty_results() {
        let fs = FakeFs::new();
        let host = FakeGitHost::new();
        let repo = FakeGitRepo::github(PathBuf::from("/repo"));
        let ctx = Context {
            fs: &fs,
            host: &host,
            repo: &repo,
        };

        run(&ctx).unwrap();
    }

    #[test]
    fn status_runs_with_issues() {
        let fs = FakeFs::new();
        let host = FakeGitHost {
            issues: vec![Issue {
                number: 42,
                title: "Add pagination".to_string(),
                url: "https://github.com/user/repo/issues/42".to_string(),
            }],
            ..FakeGitHost::new()
        };
        let repo = FakeGitRepo::github(PathBuf::from("/repo"));
        let ctx = Context {
            fs: &fs,
            host: &host,
            repo: &repo,
        };

        run(&ctx).unwrap();
    }
}
