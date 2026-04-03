use crate::context::Context;
use crate::git;
use crate::pipeline::{self, Stage, Task};
use anyhow::{Result, bail};

pub fn run(ctx: &Context, stage_filter: Option<&str>) -> Result<()> {
    let _root = ctx.repo.repo_root()?;
    let remote = ctx.repo.remote_url("origin")?;
    if !git::is_github_url(&remote) {
        bail!(
            "origin remote does not point to GitHub.\n  Found: {}",
            remote
        );
    }

    let labels = &ctx.config.labels;
    let all_labels: Vec<&str> = vec![
        &labels.needs_plan,
        &labels.plan_ready,
        &labels.plan_approved,
        &labels.review_pending,
        &labels.review_addressed,
    ];

    let issues = ctx.host.list_issues_any_label(&all_labels)?;
    let prs = ctx.host.list_prs_any_label(&all_labels)?;
    let tasks = pipeline::build_tasks(issues, prs, ctx.config);

    let filtered: Vec<&Task> = if let Some(filter) = stage_filter {
        let target = parse_stage_filter(filter)?;
        tasks.iter().filter(|t| t.stage == target).collect()
    } else {
        tasks.iter().collect()
    };

    if filtered.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    // Group by stage, open first
    let mut open: Vec<&Task> = filtered
        .iter()
        .filter(|t| t.stage.is_open())
        .copied()
        .collect();
    let mut closed: Vec<&Task> = filtered
        .iter()
        .filter(|t| !t.stage.is_open())
        .copied()
        .collect();

    // Sort by issue number descending (newest first)
    open.sort_by(|a, b| b.issue.number.cmp(&a.issue.number));
    closed.sort_by(|a, b| b.issue.number.cmp(&a.issue.number));

    let all_sorted: Vec<&Task> = open.into_iter().chain(closed).collect();

    let mut current_stage: Option<&str> = None;
    for task in all_sorted {
        let stage_name = task.stage.display();
        if current_stage != Some(stage_name) {
            if current_stage.is_some() {
                println!();
            }
            println!("\x1b[1m{stage_name}\x1b[0m");
            current_stage = Some(stage_name);
        }

        let pr_info = task
            .pr
            .as_ref()
            .map(|pr| format!("  PR #{}", pr.number))
            .unwrap_or_default();

        println!(
            "  #{:<5} {}{}",
            task.issue.number, task.issue.title, pr_info
        );
    }

    Ok(())
}

fn parse_stage_filter(s: &str) -> Result<Stage> {
    match s {
        "planning" => Ok(Stage::Planning),
        "planned" => Ok(Stage::Planned),
        "approved" => Ok(Stage::Approved),
        "coding" => Ok(Stage::Coding),
        "review" => Ok(Stage::Review),
        "done" => Ok(Stage::Done),
        "abandoned" => Ok(Stage::Abandoned),
        _ => bail!(
            "unknown stage: '{}'. Valid stages: planning, planned, approved, coding, review, done, abandoned",
            s
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DagenticConfig;
    use crate::context::Context;
    use crate::fs::fake::FakeFs;
    use crate::gh::fake::FakeGitHost;
    use crate::gh::{Issue, LabelRef};
    use crate::git::fake::FakeGitRepo;
    use std::path::PathBuf;

    fn label(name: &str) -> LabelRef {
        LabelRef {
            name: name.to_string(),
        }
    }

    fn test_ctx<'a>(
        fs: &'a FakeFs,
        host: &'a FakeGitHost,
        repo: &'a FakeGitRepo,
        config: &'a DagenticConfig,
    ) -> Context<'a> {
        Context {
            config,
            fs,
            host,
            repo,
        }
    }

    #[test]
    fn list_empty() {
        let fs = FakeFs::new();
        let host = FakeGitHost::new();
        let repo = FakeGitRepo::github(PathBuf::from("/repo"));
        let config = DagenticConfig::default();
        let ctx = test_ctx(&fs, &host, &repo, &config);

        run(&ctx, None).unwrap();
    }

    #[test]
    fn list_groups_by_stage() {
        let fs = FakeFs::new();
        let host = FakeGitHost {
            issues: vec![
                Issue {
                    number: 1,
                    title: "Plan me".to_string(),
                    url: String::new(),
                    state: "OPEN".to_string(),
                    labels: vec![label("needs-plan")],
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                },
                Issue {
                    number: 2,
                    title: "Ready for review".to_string(),
                    url: String::new(),
                    state: "OPEN".to_string(),
                    labels: vec![label("plan-ready")],
                    created_at: "2026-01-02T00:00:00Z".to_string(),
                },
            ],
            ..FakeGitHost::new()
        };
        let repo = FakeGitRepo::github(PathBuf::from("/repo"));
        let config = DagenticConfig::default();
        let ctx = test_ctx(&fs, &host, &repo, &config);

        // Should not panic, groups issues by stage
        run(&ctx, None).unwrap();
    }

    #[test]
    fn list_filters_by_stage() {
        let fs = FakeFs::new();
        let host = FakeGitHost {
            issues: vec![
                Issue {
                    number: 1,
                    title: "Plan me".to_string(),
                    url: String::new(),
                    state: "OPEN".to_string(),
                    labels: vec![label("needs-plan")],
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                },
                Issue {
                    number: 2,
                    title: "Approved".to_string(),
                    url: String::new(),
                    state: "OPEN".to_string(),
                    labels: vec![label("plan-approved")],
                    created_at: "2026-01-02T00:00:00Z".to_string(),
                },
            ],
            ..FakeGitHost::new()
        };
        let repo = FakeGitRepo::github(PathBuf::from("/repo"));
        let config = DagenticConfig::default();
        let ctx = test_ctx(&fs, &host, &repo, &config);

        run(&ctx, Some("planning")).unwrap();
    }

    #[test]
    fn parse_valid_stages() {
        assert_eq!(parse_stage_filter("planning").unwrap(), Stage::Planning);
        assert_eq!(parse_stage_filter("done").unwrap(), Stage::Done);
        assert_eq!(parse_stage_filter("review").unwrap(), Stage::Review);
    }

    #[test]
    fn parse_invalid_stage() {
        assert!(parse_stage_filter("invalid").is_err());
    }
}
