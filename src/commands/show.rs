use crate::context::Context;
use crate::git;
use crate::metadata;
use crate::pipeline;
use crate::timeline;
use anyhow::{Result, bail};

pub fn run(ctx: &Context, issue_number: u64) -> Result<()> {
    let _root = ctx.repo.repo_root()?;
    let remote = ctx.repo.remote_url("origin")?;
    if !git::is_github_url(&remote) {
        bail!(
            "origin remote does not point to GitHub.\n  Found: {}",
            remote
        );
    }

    let issue = ctx.host.get_issue(issue_number)?;
    let stage = pipeline::classify_issue(&issue, ctx.config);

    println!("\x1b[1m#{} {}\x1b[0m", issue.number, issue.title);
    println!("Stage: {}", stage.display());
    println!();

    // Timeline: stage durations
    let events = ctx.host.get_issue_timeline(issue_number)?;
    let timings = timeline::extract_stage_timings(&events, ctx.config);

    if !timings.is_empty() {
        println!("{:<25} {:<22} Duration", "Stage", "Started");
        for timing in &timings {
            let duration = match &timing.ended {
                Some(end) => timeline::format_duration(&timing.started, end),
                None => "(active)".to_string(),
            };
            let started = format_time_short(&timing.started);
            println!("{:<25} {:<22} {}", timing.label, started, duration);
        }
        println!();
    }

    // Comments: count + metadata extraction
    let comments = ctx.host.get_issue_comments(issue_number)?;
    let comment_count = comments.len();

    let mut all_metadata = Vec::new();
    for comment in &comments {
        all_metadata.extend(metadata::parse_comment(&comment.body));
    }

    // Also check PR comments if we can find a linked PR
    let labels = &ctx.config.labels;
    let pr_labels = [
        labels.review_pending.as_str(),
        labels.review_addressed.as_str(),
    ];
    if let Ok(prs) = ctx.host.list_prs_any_label(&pr_labels) {
        for pr in &prs {
            if pr.title.contains(&format!("#{issue_number}")) {
                println!("Linked PR: #{} {}", pr.number, pr.title);
                if let Some(merged) = &pr.merged_at {
                    println!("Merged: {}", format_time_short(merged));
                }

                if let Ok(pr_comments) = ctx.host.get_pr_comments(pr.number) {
                    for comment in &pr_comments {
                        all_metadata.extend(metadata::parse_comment(&comment.body));
                    }
                }
                println!();
                break;
            }
        }
    }

    // Print metadata stats if any
    if !all_metadata.is_empty() {
        println!("Agent stats:");
        for m in &all_metadata {
            let tokens = match (m.tokens_in, m.tokens_out) {
                (Some(i), Some(o)) => format!("{} in / {} out", i, o),
                (Some(i), None) => format!("{} in", i),
                (None, Some(o)) => format!("{} out", o),
                (None, None) => "no token data".to_string(),
            };
            let model = m.model.as_deref().unwrap_or("unknown");
            println!("  {:<20} {} ({})", m.phase, tokens, model);
        }
        println!();
    }

    println!("Comments on issue: {comment_count}");

    Ok(())
}

fn format_time_short(iso: &str) -> String {
    // "2026-04-01T10:45:00Z" → "Apr 1 10:45"
    let ts = iso.trim_end_matches('Z');
    let (date, time) = match ts.split_once('T') {
        Some(pair) => pair,
        None => return iso.to_string(),
    };
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return iso.to_string();
    }
    let month = match parts[1] {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => parts[1],
    };
    let day: u32 = parts[2].parse().unwrap_or(0);
    let time_short = &time[..5]; // HH:MM
    format!("{month} {day} {time_short}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DagenticConfig;
    use crate::context::Context;
    use crate::fs::fake::FakeFs;
    use crate::gh::fake::FakeGitHost;
    use crate::gh::{Comment, Issue, LabelRef, TimelineEvent, TimelineLabel};
    use crate::git::fake::FakeGitRepo;
    use std::path::PathBuf;

    fn label(name: &str) -> LabelRef {
        LabelRef {
            name: name.to_string(),
        }
    }

    fn labeled_event(name: &str, time: &str) -> TimelineEvent {
        TimelineEvent {
            event: Some("labeled".to_string()),
            label: Some(TimelineLabel {
                name: name.to_string(),
            }),
            created_at: Some(time.to_string()),
        }
    }

    #[test]
    fn show_basic_issue() {
        let fs = FakeFs::new();
        let host = FakeGitHost {
            issues: vec![Issue {
                number: 1,
                title: "Add pagination".to_string(),
                url: String::new(),
                state: "OPEN".to_string(),
                labels: vec![label("needs-plan")],
                created_at: "2026-04-01T10:00:00Z".to_string(),
            }],
            timeline: vec![labeled_event("needs-plan", "2026-04-01T10:00:00Z")],
            ..FakeGitHost::new()
        };
        let repo = FakeGitRepo::github(PathBuf::from("/repo"));
        let config = DagenticConfig::default();
        let ctx = Context {
            config: &config,
            fs: &fs,
            host: &host,
            repo: &repo,
        };

        run(&ctx, 1).unwrap();
    }

    #[test]
    fn show_issue_with_metadata() {
        let fs = FakeFs::new();
        let host = FakeGitHost {
            issues: vec![Issue {
                number: 5,
                title: "Fix auth".to_string(),
                url: String::new(),
                state: "OPEN".to_string(),
                labels: vec![label("plan-ready")],
                created_at: "2026-04-01T10:00:00Z".to_string(),
            }],
            comments: vec![Comment {
                body: "Plan looks good.\n<!-- dagentic:phase=planning tokens_in=5000 tokens_out=2000 model=claude-opus-4-6 -->".to_string(),
                created_at: "2026-04-01T10:30:00Z".to_string(),
                author: None,
            }],
            timeline: vec![
                labeled_event("needs-plan", "2026-04-01T10:00:00Z"),
                labeled_event("plan-ready", "2026-04-01T10:30:00Z"),
            ],
            ..FakeGitHost::new()
        };
        let repo = FakeGitRepo::github(PathBuf::from("/repo"));
        let config = DagenticConfig::default();
        let ctx = Context {
            config: &config,
            fs: &fs,
            host: &host,
            repo: &repo,
        };

        run(&ctx, 5).unwrap();
    }

    #[test]
    fn show_nonexistent_issue_errors() {
        let fs = FakeFs::new();
        let host = FakeGitHost::new();
        let repo = FakeGitRepo::github(PathBuf::from("/repo"));
        let config = DagenticConfig::default();
        let ctx = Context {
            config: &config,
            fs: &fs,
            host: &host,
            repo: &repo,
        };

        assert!(run(&ctx, 999).is_err());
    }

    #[test]
    fn format_time_short_works() {
        assert_eq!(format_time_short("2026-04-01T10:45:00Z"), "Apr 1 10:45");
        assert_eq!(format_time_short("2026-12-25T08:00:00Z"), "Dec 25 08:00");
    }
}
