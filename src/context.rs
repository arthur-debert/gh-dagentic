use crate::fs::Filesystem;
use crate::gh::GitHost;
use crate::git::GitRepo;

pub struct Context<'a> {
    pub fs: &'a dyn Filesystem,
    pub host: &'a dyn GitHost,
    pub repo: &'a dyn GitRepo,
}
