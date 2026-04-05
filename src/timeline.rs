/// Compute stage durations from GitHub issue timeline events.
use crate::config::DagenticConfig;
use crate::gh::TimelineEvent;

#[derive(Debug, Clone)]
pub struct StageTiming {
    pub label: String,
    pub started: String,
    pub ended: Option<String>,
}

/// Extract label-add events relevant to dagentic, in chronological order.
pub fn extract_stage_timings(
    events: &[TimelineEvent],
    config: &DagenticConfig,
) -> Vec<StageTiming> {
    let dagentic_labels: Vec<&str> = vec![
        &config.labels.needs_plan,
        &config.labels.plan_ready,
        &config.labels.plan_approved,
        &config.labels.review_pending,
        &config.labels.review_addressed,
    ];

    let mut timings: Vec<StageTiming> = Vec::new();

    for event in events {
        let event_type = match &event.event {
            Some(e) => e.as_str(),
            None => continue,
        };
        let label_name = match &event.label {
            Some(l) => l.name.as_str(),
            None => continue,
        };
        let timestamp = match &event.created_at {
            Some(t) => t.clone(),
            None => continue,
        };

        if !dagentic_labels.contains(&label_name) {
            continue;
        }

        if event_type == "labeled" {
            // Close the previous stage
            if let Some(prev) = timings.last_mut()
                && prev.ended.is_none()
            {
                prev.ended = Some(timestamp.clone());
            }
            timings.push(StageTiming {
                label: label_name.to_string(),
                started: timestamp,
                ended: None,
            });
        }
    }

    timings
}

/// Format an ISO 8601 duration between two timestamps as a human-readable string.
pub fn format_duration(start: &str, end: &str) -> String {
    let start_secs = parse_timestamp(start);
    let end_secs = parse_timestamp(end);
    match (start_secs, end_secs) {
        (Some(s), Some(e)) if e > s => format_seconds(e - s),
        _ => "?".to_string(),
    }
}

/// Format seconds as human-readable duration.
pub fn format_seconds(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        if s == 0 {
            format!("{m}min")
        } else {
            format!("{m}min {s}s")
        }
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        if m == 0 {
            format!("{h}h")
        } else {
            format!("{h}h {m}min")
        }
    }
}

/// Parse ISO 8601 timestamp to Unix seconds. Simple parser for GitHub's format.
fn parse_timestamp(ts: &str) -> Option<u64> {
    // GitHub format: "2026-04-03T17:28:32Z"
    let ts = ts.trim_end_matches('Z');
    let (date, time) = ts.split_once('T')?;
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: u64 = parts[0].parse().ok()?;
    let month: u64 = parts[1].parse().ok()?;
    let day: u64 = parts[2].parse().ok()?;

    let time_parts: Vec<&str> = time.split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hour: u64 = time_parts[0].parse().ok()?;
    let min: u64 = time_parts[1].parse().ok()?;
    let sec: u64 = time_parts[2].parse().ok()?;

    // Approximate days since epoch (good enough for duration diffs)
    let days = (year - 1970) * 365 + (year - 1969) / 4 + days_before_month(month, year) + day - 1;
    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}

fn days_before_month(month: u64, year: u64) -> u64 {
    let leap = if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
    {
        1
    } else {
        0
    };
    match month {
        1 => 0,
        2 => 31,
        3 => 59 + leap,
        4 => 90 + leap,
        5 => 120 + leap,
        6 => 151 + leap,
        7 => 181 + leap,
        8 => 212 + leap,
        9 => 243 + leap,
        10 => 273 + leap,
        11 => 304 + leap,
        12 => 334 + leap,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DagenticConfig;
    use crate::gh::{TimelineEvent, TimelineLabel};

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
    fn extract_planning_to_planned() {
        let config = DagenticConfig::default();
        let events = vec![
            labeled_event("needs-plan", "2026-04-01T10:00:00Z"),
            labeled_event("plan-ready", "2026-04-01T10:45:00Z"),
        ];
        let timings = extract_stage_timings(&events, &config);

        assert_eq!(timings.len(), 2);
        assert_eq!(timings[0].label, "needs-plan");
        assert_eq!(timings[0].ended.as_deref(), Some("2026-04-01T10:45:00Z"));
        assert_eq!(timings[1].label, "plan-ready");
        assert!(timings[1].ended.is_none()); // still open
    }

    #[test]
    fn extract_full_pipeline() {
        let config = DagenticConfig::default();
        let events = vec![
            labeled_event("needs-plan", "2026-04-01T10:00:00Z"),
            labeled_event("plan-ready", "2026-04-01T10:30:00Z"),
            labeled_event("plan-approved", "2026-04-01T12:00:00Z"),
            labeled_event("review-pending", "2026-04-01T13:30:00Z"),
        ];
        let timings = extract_stage_timings(&events, &config);

        assert_eq!(timings.len(), 4);
        // Each stage except last should have an end time
        for t in &timings[..3] {
            assert!(t.ended.is_some(), "stage {} should be closed", t.label);
        }
        assert!(timings[3].ended.is_none());
    }

    #[test]
    fn ignores_non_dagentic_labels() {
        let config = DagenticConfig::default();
        let events = vec![
            labeled_event("needs-plan", "2026-04-01T10:00:00Z"),
            labeled_event("priority:high", "2026-04-01T10:05:00Z"),
            labeled_event("plan-ready", "2026-04-01T10:30:00Z"),
        ];
        let timings = extract_stage_timings(&events, &config);
        assert_eq!(timings.len(), 2);
    }

    #[test]
    fn format_seconds_various() {
        assert_eq!(format_seconds(30), "30s");
        assert_eq!(format_seconds(90), "1min 30s");
        assert_eq!(format_seconds(3600), "1h");
        assert_eq!(format_seconds(5400), "1h 30min");
        assert_eq!(format_seconds(7200), "2h");
    }

    #[test]
    fn format_duration_between_timestamps() {
        assert_eq!(
            format_duration("2026-04-01T10:00:00Z", "2026-04-01T10:45:00Z"),
            "45min"
        );
        assert_eq!(
            format_duration("2026-04-01T10:00:00Z", "2026-04-01T12:30:00Z"),
            "2h 30min"
        );
    }
}
