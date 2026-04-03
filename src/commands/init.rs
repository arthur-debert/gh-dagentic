use crate::context::Context;
use crate::git;
use crate::labels;
use crate::templates::{self, TemplateSet};
use anyhow::{Result, bail};

pub fn run(ctx: &Context) -> Result<()> {
    println!("\x1b[1mInitializing Dagentic...\x1b[0m\n");

    let root = ctx.repo.repo_root()?;
    let remote = ctx.repo.remote_url("origin")?;
    if !git::is_github_url(&remote) {
        bail!("origin remote does not point to GitHub.\n  Found: {}", remote);
    }

    println!("Installing workflow files...");
    let caller_result = templates::install(ctx.fs, &TemplateSet::Caller, &root)?;
    print_install_result(&caller_result);

    println!("\nInstalling issue templates...");
    let issue_result = templates::install(ctx.fs, &TemplateSet::Issue, &root)?;
    print_install_result(&issue_result);

    println!("\nCreating labels...");
    for (name, result) in labels::create_all(ctx.host) {
        match result {
            Ok(()) => println!("  {name}"),
            Err(e) => eprintln!("  Warning: could not create label '{name}': {e}"),
        }
    }

    println!("\nChecking secrets...");
    match ctx.host.list_secrets() {
        Ok(secrets) => {
            if secrets.iter().any(|s| s.name == "ANTHROPIC_API_KEY") {
                println!("  ANTHROPIC_API_KEY is configured.");
            } else {
                println!("  ANTHROPIC_API_KEY is not set. Set it with:\n");
                println!("    gh secret set ANTHROPIC_API_KEY\n");
                println!("  You'll be prompted to paste your key.");
                println!("  Get one at https://console.anthropic.com/settings/keys");
            }
        }
        Err(_) => {
            println!("  Could not check secrets (you may not have admin access).");
            println!("  Ensure ANTHROPIC_API_KEY is set as a repository secret.");
        }
    }

    println!();
    let claude_md = root.join("CLAUDE.md");
    if ctx.fs.file_exists(&claude_md) {
        println!("CLAUDE.md found. The agents will use it for project-specific instructions.");
    } else {
        println!("No CLAUDE.md found. Consider creating one with your project conventions");
        println!("(branching, testing, code style). The agents read it before every task.");
    }

    println!("\n\x1b[1mDone.\x1b[0m Create an issue using one of the templates to start the pipeline.");
    Ok(())
}

fn print_install_result(result: &templates::InstallResult) {
    for name in &result.created {
        println!("  Created {name}");
    }
    for name in &result.updated {
        println!("  Updated {name}");
    }
    for name in &result.unchanged {
        println!("  \x1b[2mUnchanged {name}\x1b[0m");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use crate::fs::fake::FakeFs;
    use crate::gh::fake::FakeGitHost;
    use crate::git::fake::FakeGitRepo;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from("/repo")
    }

    fn make_ctx<'a>(
        fs: &'a FakeFs,
        host: &'a FakeGitHost,
        repo: &'a FakeGitRepo,
    ) -> Context<'a> {
        Context { fs, host, repo }
    }

    #[test]
    fn init_creates_all_templates_and_labels() {
        let fs = FakeFs::new();
        let host = FakeGitHost::new().with_secret("ANTHROPIC_API_KEY");
        let repo = FakeGitRepo::github(root());
        let ctx = make_ctx(&fs, &host, &repo);

        run(&ctx).unwrap();

        // 4 caller + 3 issue templates
        assert_eq!(fs.files.borrow().len(), 7);

        // All 8 labels created
        assert_eq!(host.labels_created.borrow().len(), 8);
    }

    #[test]
    fn init_writes_caller_templates_to_workflows_dir() {
        let fs = FakeFs::new();
        let host = FakeGitHost::new();
        let repo = FakeGitRepo::github(root());
        let ctx = make_ctx(&fs, &host, &repo);

        run(&ctx).unwrap();

        let files = fs.files.borrow();
        assert!(files.contains_key(&root().join(".github/workflows/main-agent-plan.yml")));
        assert!(files.contains_key(&root().join(".github/workflows/main-agent-implement.yml")));
        assert!(files.contains_key(&root().join(".github/workflows/main-agent-review-fixup.yml")));
        assert!(files.contains_key(&root().join(".github/workflows/side-agent-review.yml")));
    }

    #[test]
    fn init_writes_issue_templates() {
        let fs = FakeFs::new();
        let host = FakeGitHost::new();
        let repo = FakeGitRepo::github(root());
        let ctx = make_ctx(&fs, &host, &repo);

        run(&ctx).unwrap();

        let files = fs.files.borrow();
        assert!(files.contains_key(&root().join(".github/ISSUE_TEMPLATE/bug.yml")));
        assert!(files.contains_key(&root().join(".github/ISSUE_TEMPLATE/feature.yml")));
        assert!(files.contains_key(&root().join(".github/ISSUE_TEMPLATE/epic.yml")));
    }

    #[test]
    fn init_fails_on_non_github_remote() {
        let fs = FakeFs::new();
        let host = FakeGitHost::new();
        let repo = FakeGitRepo {
            root: root(),
            remote: "git@gitlab.com:user/repo.git".to_string(),
        };
        let ctx = make_ctx(&fs, &host, &repo);

        let err = run(&ctx).unwrap_err();
        assert!(err.to_string().contains("GitHub"));
    }

    #[test]
    fn init_detects_claude_md() {
        let fs = FakeFs::new().with_file(root().join("CLAUDE.md"), "# My project");
        let host = FakeGitHost::new();
        let repo = FakeGitRepo::github(root());
        let ctx = make_ctx(&fs, &host, &repo);

        // Should not error — CLAUDE.md is found
        run(&ctx).unwrap();
    }
}
