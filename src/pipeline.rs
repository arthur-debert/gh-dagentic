/// Pipeline stage classification based on current labels.
use crate::config::DagenticConfig;
use crate::gh::{Issue, PullRequest};

#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    Planning,
    Planned,
    Approved,
    Coding,
    Review,
    ReviewAddressed,
    Done,
    Abandoned,
}

impl Stage {
    pub fn display(&self) -> &'static str {
        match self {
            Self::Planning => "Planning",
            Self::Planned => "Planned (awaiting approval)",
            Self::Approved => "Approved (awaiting implementation)",
            Self::Coding => "Coding",
            Self::Review => "In review",
            Self::ReviewAddressed => "Review addressed",
            Self::Done => "Done",
            Self::Abandoned => "Abandoned",
        }
    }

    pub fn is_open(&self) -> bool {
        !matches!(self, Self::Done | Self::Abandoned)
    }
}

pub fn classify_issue(issue: &Issue, config: &DagenticConfig) -> Stage {
    let labels = &issue.labels;
    let has = |name: &str| labels.iter().any(|l| l.name == name);

    if has(&config.labels.plan_approved) {
        Stage::Approved
    } else if has(&config.labels.plan_ready) {
        Stage::Planned
    } else if has(&config.labels.needs_plan) {
        Stage::Planning
    } else if issue.state == "CLOSED" {
        Stage::Abandoned
    } else {
        Stage::Planning // fallback for issues with dagentic labels
    }
}

pub fn classify_pr(pr: &PullRequest, config: &DagenticConfig) -> Stage {
    let labels = &pr.labels;
    let has = |name: &str| labels.iter().any(|l| l.name == name);

    if pr.merged_at.is_some() {
        Stage::Done
    } else if pr.state == "CLOSED" {
        Stage::Abandoned
    } else if has(&config.labels.review_addressed) {
        Stage::ReviewAddressed
    } else if has(&config.labels.review_pending) {
        Stage::Review
    } else {
        Stage::Coding
    }
}

/// A unified view of a dagentic task: an issue optionally linked to a PR.
#[derive(Debug, Clone)]
pub struct Task {
    pub issue: Issue,
    pub pr: Option<PullRequest>,
    pub stage: Stage,
}

/// Build tasks by matching issues to PRs. PRs reference issues via title convention
/// or we fall back to label-based stage from the issue alone.
pub fn build_tasks(
    issues: Vec<Issue>,
    prs: Vec<PullRequest>,
    config: &DagenticConfig,
) -> Vec<Task> {
    let mut tasks: Vec<Task> = Vec::new();

    for issue in issues {
        // Find a PR that mentions this issue number in title or body
        let linked_pr = prs.iter().find(|pr| {
            pr.title.contains(&format!("#{}", issue.number))
                || pr.title.contains(&format!("#{}", issue.number))
        });

        let stage = if let Some(pr) = linked_pr {
            classify_pr(pr, config)
        } else {
            classify_issue(&issue, config)
        };

        tasks.push(Task {
            issue,
            pr: linked_pr.cloned(),
            stage,
        });
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DagenticConfig;
    use crate::gh::LabelRef;

    fn label(name: &str) -> LabelRef {
        LabelRef {
            name: name.to_string(),
        }
    }

    fn issue(number: u64, labels: Vec<LabelRef>, state: &str) -> Issue {
        Issue {
            number,
            title: format!("Issue {number}"),
            url: String::new(),
            state: state.to_string(),
            labels,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn classify_planning_issue() {
        let config = DagenticConfig::default();
        let i = issue(1, vec![label("needs-plan")], "OPEN");
        assert_eq!(classify_issue(&i, &config), Stage::Planning);
    }

    #[test]
    fn classify_planned_issue() {
        let config = DagenticConfig::default();
        let i = issue(1, vec![label("plan-ready")], "OPEN");
        assert_eq!(classify_issue(&i, &config), Stage::Planned);
    }

    #[test]
    fn classify_approved_issue() {
        let config = DagenticConfig::default();
        let i = issue(1, vec![label("plan-approved")], "OPEN");
        assert_eq!(classify_issue(&i, &config), Stage::Approved);
    }

    #[test]
    fn classify_closed_issue_as_abandoned() {
        let config = DagenticConfig::default();
        let i = issue(1, vec![], "CLOSED");
        assert_eq!(classify_issue(&i, &config), Stage::Abandoned);
    }

    #[test]
    fn classify_merged_pr() {
        let config = DagenticConfig::default();
        let pr = PullRequest {
            number: 10,
            title: "Fix #1".to_string(),
            url: String::new(),
            state: "MERGED".to_string(),
            labels: vec![],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            merged_at: Some("2026-01-02T00:00:00Z".to_string()),
        };
        assert_eq!(classify_pr(&pr, &config), Stage::Done);
    }

    #[test]
    fn classify_pr_in_review() {
        let config = DagenticConfig::default();
        let pr = PullRequest {
            number: 10,
            title: "Fix #1".to_string(),
            url: String::new(),
            state: "OPEN".to_string(),
            labels: vec![label("review-pending")],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            merged_at: None,
        };
        assert_eq!(classify_pr(&pr, &config), Stage::Review);
    }

    #[test]
    fn build_tasks_links_issue_to_pr() {
        let config = DagenticConfig::default();
        let issues = vec![issue(5, vec![label("plan-approved")], "OPEN")];
        let prs = vec![PullRequest {
            number: 10,
            title: "Implement #5".to_string(),
            url: String::new(),
            state: "OPEN".to_string(),
            labels: vec![label("review-pending")],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            merged_at: None,
        }];

        let tasks = build_tasks(issues, prs, &config);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].stage, Stage::Review);
        assert!(tasks[0].pr.is_some());
    }
}
