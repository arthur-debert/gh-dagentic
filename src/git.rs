use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::process::Command;

pub trait GitRepo {
    fn repo_root(&self) -> Result<PathBuf>;
    fn remote_url(&self, name: &str) -> Result<String>;
}

pub fn is_github_url(url: &str) -> bool {
    url.contains("github.com")
}

pub struct GitCli;

impl GitRepo for GitCli {
    fn repo_root(&self) -> Result<PathBuf> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .context("failed to run git")?;
        if !output.status.success() {
            bail!("not inside a git repository");
        }
        let path = String::from_utf8(output.stdout)
            .context("invalid utf-8 in git output")?
            .trim()
            .to_string();
        Ok(PathBuf::from(path))
    }

    fn remote_url(&self, name: &str) -> Result<String> {
        let output = Command::new("git")
            .args(["remote", "get-url", name])
            .output()
            .context("failed to run git")?;
        if !output.status.success() {
            bail!("no '{}' remote found", name);
        }
        Ok(String::from_utf8(output.stdout)
            .context("invalid utf-8 in git output")?
            .trim()
            .to_string())
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;

    pub struct FakeGitRepo {
        pub root: PathBuf,
        pub remote: String,
    }

    impl FakeGitRepo {
        pub fn github(root: impl Into<PathBuf>) -> Self {
            Self {
                root: root.into(),
                remote: "git@github.com:user/repo.git".to_string(),
            }
        }
    }

    impl GitRepo for FakeGitRepo {
        fn repo_root(&self) -> Result<PathBuf> {
            Ok(self.root.clone())
        }

        fn remote_url(&self, _name: &str) -> Result<String> {
            Ok(self.remote.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_ssh_url() {
        assert!(is_github_url("git@github.com:user/repo.git"));
    }

    #[test]
    fn github_https_url() {
        assert!(is_github_url("https://github.com/user/repo.git"));
    }

    #[test]
    fn non_github_url() {
        assert!(!is_github_url("git@gitlab.com:user/repo.git"));
        assert!(!is_github_url("https://bitbucket.org/user/repo.git"));
    }
}
