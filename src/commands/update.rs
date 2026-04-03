use crate::context::Context;
use crate::git;
use crate::labels;
use crate::templates::{self, TemplateSet};
use anyhow::{Result, bail};

pub fn run(ctx: &Context) -> Result<()> {
    println!("\x1b[1mUpdating Dagentic files...\x1b[0m\n");

    let root = ctx.repo.repo_root()?;
    let remote = ctx.repo.remote_url("origin")?;
    if !git::is_github_url(&remote) {
        bail!(
            "origin remote does not point to GitHub.\n  Found: {}",
            remote
        );
    }

    println!("Updating workflow files...");
    let caller_result = templates::install(ctx.fs, &TemplateSet::Caller, &root)?;
    print_install_result(&caller_result);

    println!("\nUpdating issue templates...");
    let issue_result = templates::install(ctx.fs, &TemplateSet::Issue, &root)?;
    print_install_result(&issue_result);

    println!("\nSyncing labels...");
    for (name, result) in labels::create_all(ctx.host) {
        match result {
            Ok(()) => println!("  {name}"),
            Err(e) => eprintln!("  Warning: could not create label '{name}': {e}"),
        }
    }

    let total = caller_result.changed_count() + issue_result.changed_count();
    println!();
    if total == 0 {
        println!("Everything is up to date.");
    } else {
        println!("{total} file(s) updated. Review and commit the changes.");
    }

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
    use crate::commands::init;
    use crate::context::Context;
    use crate::fs::Filesystem;
    use crate::fs::fake::FakeFs;
    use crate::gh::fake::FakeGitHost;
    use crate::git::fake::FakeGitRepo;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from("/repo")
    }

    #[test]
    fn update_after_init_reports_no_changes() {
        let fs = FakeFs::new();
        let host = FakeGitHost::new();
        let repo = FakeGitRepo::github(root());
        let ctx = Context {
            fs: &fs,
            host: &host,
            repo: &repo,
        };

        init::run(&ctx).unwrap();
        // Second run should find everything unchanged
        run(&ctx).unwrap();
    }

    #[test]
    fn update_after_tamper_reports_changes() {
        let fs = FakeFs::new();
        let host = FakeGitHost::new();
        let repo = FakeGitRepo::github(root());
        let ctx = Context {
            fs: &fs,
            host: &host,
            repo: &repo,
        };

        init::run(&ctx).unwrap();

        // Tamper with a file
        let path = root().join(".github/workflows/main-agent-plan.yml");
        fs.write_file(&path, b"modified").unwrap();

        run(&ctx).unwrap();

        // File should be restored to original content
        let contents = fs.read_file(&path).unwrap();
        assert_ne!(contents, b"modified");
    }
}
