# Dagentic

Dagentic is a simple yet capable system for orchestrating a fully fledged agentic development workflow that is easy to run, manage, and reason about.

Its core feature is simplicity: freeing you from the manual, repetitive cycle of original idea / problem, planning, reviewed planning, approval, development, code review, review fixes, final review, and human merge.

**Features:**

- Only requires a GitHub repo with a side agent for code review and your primary agent's API key.
- Leverages GitHub for workflow orchestration, logging, and events (PRs, issues, branches).
- High-level human touchpoints: initial task description, plan approval, and final merge. The first two happen on GitHub issues with predefined labels; the last one at the PR.
- Planning, execution, and review use different models for best-of-breed results and to avoid single-model bias missing issues.
- Base prompts plus project-specific additions via your repo's `CLAUDE.md`.

Agentic coding workflow orchestration can be incredibly feature-rich, configurable, and complex. Dagentic isn't any of that. Instead it is:

- A core, no-nonsense, predefined workflow that minimizes user input but keeps you in the loop at the critical points: defining the task, approving the plan, and merging the result.
- GitHub issues are used to trigger planning (by adding the `status: needs-plan` label). Issue comments iterate agent/human clarifications and changes to the plan, and a label signals ready for development.
- The final work, including a secondary agent review and fixup, ends up as a GitHub PR for you to merge. PR comments are used to request clarifications and changes.

## Agents

Dagentic uses two agents with distinct roles:

- **Primary agent** (currently [Claude Code](https://docs.anthropic.com/en/docs/claude-code)): handles planning, implementation, and review fixups. Uses your `ANTHROPIC_API_KEY`.
- **Side agent** (currently [GitHub Copilot](https://docs.github.com/en/copilot)): provides an independent code review on every PR. Requires Copilot Enterprise or Business on the repo.

Using separate agents for implementation and review avoids single-model blind spots.

## How it works

You open a GitHub issue describing what you want. From there, the pipeline takes over:

```
You create issue        "Add pagination to the API"
        |
        v
  [status: needs-plan]  (auto-labeled by issue template)
        |
        v
  Planning agent         Primary agent reads the issue, posts a detailed
  posts plan comment     plan as a comment, labels the issue plan-ready.
        |
        v
  [status: plan-ready]
        |
    You review the plan. Comment to iterate.
    When satisfied, swap the label:
        |
        v
  [status: plan-approved]
        |
        v
  Implementation agent   Primary agent creates a branch, implements
  opens draft PR         the plan, and opens a draft PR.
        |
        v
  [pr: review-pending]
        |
        v
  Side agent review      Side agent is automatically requested as a
                         reviewer on the PR.
        |
        v
  Review fixup agent     Primary agent reads review comments, pushes
                         fixes or replies with reasoning.
        |
        v
  You review and merge   The PR is yours to approve and merge.
```

Three points require your attention. Everything else is automatic:

1. **Write the issue** -- describe what you want built or fixed.
2. **Approve the plan** -- swap the label from `plan-ready` to `plan-approved`.
3. **Merge the PR** -- review the final result and merge.

## Requirements

- A GitHub repository (public or private).
- An `ANTHROPIC_API_KEY` stored as a repository secret.
- Copilot code review enabled on the repo (requires Copilot Enterprise or Business).

## Setup

### 1. Install the CLI

```bash
gh extension install arthur-debert/gh-dagentic
```

### 2. Initialize your repo

```bash
gh dagentic init
```

This creates the required labels, copies the workflow files and issue templates into your repo, and guides you through setting your API key.

### 3. Add a CLAUDE.md

The primary agent reads your repo's `CLAUDE.md` for project-specific conventions: branching strategy, testing commands, code style, and anything else the agent should know. See the [example](https://github.com/arthur-debert/seer/blob/main/CLAUDE.md) for the expected format.

### 4. Create your first issue

Use one of the issue templates (feature, bug, or epic). The `status: needs-plan` label is applied automatically, and the pipeline starts.

## Architecture

Dagentic is built entirely on GitHub Actions reusable workflows. Your repo contains thin caller workflows that trigger on label and PR events. These call the reusable workflows hosted in this repository, which do the actual work.

| Phase | Agent | What happens |
|-------|-------|-------------|
| Planning | Primary (Claude Opus) | Reads issue, posts plan comment, swaps labels |
| Implementation | Primary (Claude Sonnet) | Creates branch, implements plan, opens draft PR |
| Code review | Side (GitHub Copilot) | Requested automatically as PR reviewer |
| Review fixup | Primary (Claude Sonnet) | Addresses review comments, pushes fixes |

All compute runs on your GitHub Actions runners. All API calls use your `ANTHROPIC_API_KEY`. Nothing runs on or bills to Dagentic's infrastructure.

The caller workflows in your repo are intentionally thin -- just event triggers and a `uses:` reference to this repo. When Dagentic is updated, your repo picks up changes automatically (callers reference `@main`). Pin to a release tag if you prefer stability over freshness.

### The review-fixup relay

The review-fixup caller uses a two-stage relay pattern. Stage 1 runs on `pull_request_review` events and dispatches a `workflow_dispatch`. Stage 2 calls the reusable workflow. This works around [claude-code-action#900](https://github.com/anthropics/claude-code-action/issues/900) where bot actors are blocked before `allowed_bots` is checked.

## Labels

Dagentic uses labels to drive the workflow. These are created automatically by `gh dagentic init`:

| Label | Purpose |
|-------|---------|
| `status: needs-plan` | Triggers the planning agent |
| `status: plan-ready` | Plan posted, waiting for your review |
| `status: plan-approved` | You approved the plan, triggers implementation |
| `pr: review-pending` | Draft PR opened, triggers side agent review |
| `pr: review-addressed` | Review comments addressed |
| `type: feature` | Issue type |
| `type: bug` | Issue type |
| `type: epic` | Issue type (multi-PR planning) |
