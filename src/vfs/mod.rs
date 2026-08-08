mod disk;
mod error;
mod git;
mod git_ref;
mod workspace;

pub use error::FsError;
pub use git_ref::GitRef;
pub use workspace::Workspace;

use disk::DiskFs;
use git::GitTreeFs;

use std::path::{Component, Path, PathBuf};

/// A read-only view rooted at some location: either the live disk, or a resolved git commit's tree.
pub enum Fs<'a> {
    Disk(DiskFs),
    Git(GitTreeFs<'a>),
}

impl Fs<'_> {
    pub fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        match self {
            Fs::Disk(fs) => fs.read(path),
            Fs::Git(fs) => fs.read(path),
        }
    }

    pub fn list_dir(&self, path: &Path) -> Result<Vec<DirEntry>, FsError> {
        match self {
            Fs::Disk(fs) => fs.list_dir(path),
            Fs::Git(fs) => fs.list_dir(path),
        }
    }

    pub fn exists(&self, path: &Path) -> bool {
        match self {
            Fs::Disk(fs) => fs.exists(path),
            Fs::Git(fs) => fs.exists(path),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub kind: EntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Dir,
}

pub(crate) fn normalize_relative(path: &Path) -> Result<PathBuf, FsError> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
            Component::ParentDir => return Err(FsError::EscapesRoot(path.to_path_buf())),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod test_support {
    use git2::{Oid, Repository, Signature};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::GitRef;

    pub fn head_ref(repo: &Repository) -> GitRef {
        GitRef {
            oid: repo.head().unwrap().peel_to_commit().unwrap().id(),
        }
    }

    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new(label: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "opsxexplorer-vfs-test-{label}-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    pub fn write_file(root: &Path, rel: &str, content: &str) {
        let full = root.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full, content).unwrap();
    }

    pub fn stage_and_commit(repo: &Repository, message: &str, paths: &[&str]) -> Oid {
        let mut index = repo.index().unwrap();
        for path in paths {
            index.add_path(Path::new(path)).unwrap();
        }
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("Test", "test@example.com").unwrap();
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use git2::Repository;

    #[test]
    fn discover_nested_path_roots_at_repo_root() {
        let dir = TempDir::new("discover-nested");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(dir.path(), "top.txt", "top-level");
        write_file(dir.path(), "sub/dir/nested.txt", "nested");
        stage_and_commit(&repo, "initial", &["top.txt", "sub/dir/nested.txt"]);

        let nested_start = dir.path().join("sub").join("dir");
        let workspace = Workspace::open(&nested_start).unwrap();

        let view = workspace.current();
        assert!(view.exists(Path::new("top.txt")));
        assert!(view.exists(Path::new("sub/dir/nested.txt")));
    }

    #[test]
    fn no_repo_roots_exactly_at_requested_path() {
        let dir = TempDir::new("no-repo");
        write_file(dir.path(), "readme.md", "hello");

        let workspace = Workspace::open(dir.path()).unwrap();
        let view = workspace.current();

        assert_eq!(view.read(Path::new("readme.md")).unwrap(), b"hello");
        let dummy_ref = GitRef {
            oid: git2::Oid::ZERO_SHA1,
        };
        assert!(matches!(
            workspace.at(&dummy_ref),
            Err(FsError::NotAGitRepo)
        ));
    }

    #[test]
    fn disk_view_reads_and_lists() {
        let dir = TempDir::new("disk-read-list");
        write_file(dir.path(), "a.txt", "aaa");
        write_file(dir.path(), "dir/b.txt", "bbb");

        let workspace = Workspace::open(dir.path()).unwrap();
        let view = workspace.current();

        assert_eq!(view.read(Path::new("a.txt")).unwrap(), b"aaa");
        let entries = view.list_dir(Path::new(".")).unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["a.txt", "dir"]);
        assert_eq!(entries[1].kind, EntryKind::Dir);
    }

    #[test]
    fn disk_view_rejects_traversal_outside_root() {
        let dir = TempDir::new("disk-traversal");
        write_file(dir.path(), "inside.txt", "inside");

        let workspace = Workspace::open(dir.path()).unwrap();
        let view = workspace.current();

        let result = view.read(Path::new("../outside.txt"));
        assert!(matches!(result, Err(FsError::EscapesRoot(_))));
    }

    #[test]
    fn working_view_shows_uncommitted_untracked_and_gitignored() {
        let dir = TempDir::new("working-view");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(dir.path(), "tracked.txt", "v1");
        write_file(dir.path(), ".gitignore", "ignored.txt\n");
        stage_and_commit(&repo, "initial", &["tracked.txt", ".gitignore"]);

        // Uncommitted edit.
        write_file(dir.path(), "tracked.txt", "v2-uncommitted");
        // Untracked file.
        write_file(dir.path(), "untracked.txt", "untracked");
        // Gitignored file.
        write_file(dir.path(), "ignored.txt", "ignored-content");

        let workspace = Workspace::open(dir.path()).unwrap();
        let view = workspace.current();

        assert_eq!(
            view.read(Path::new("tracked.txt")).unwrap(),
            b"v2-uncommitted"
        );
        assert_eq!(view.read(Path::new("untracked.txt")).unwrap(), b"untracked");
        assert_eq!(
            view.read(Path::new("ignored.txt")).unwrap(),
            b"ignored-content"
        );
    }

    #[test]
    fn ref_view_unaffected_by_working_tree_edits() {
        let dir = TempDir::new("ref-vs-working");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(dir.path(), "file.txt", "v1");
        stage_and_commit(&repo, "v1", &["file.txt"]);

        let workspace = Workspace::open(dir.path()).unwrap();
        let v1_ref = head_ref(&repo);

        write_file(dir.path(), "file.txt", "v2-uncommitted");

        let v1_view = workspace.at(&v1_ref).unwrap();
        assert_eq!(v1_view.read(Path::new("file.txt")).unwrap(), b"v1");

        let current_view = workspace.current();
        assert_eq!(
            current_view.read(Path::new("file.txt")).unwrap(),
            b"v2-uncommitted"
        );
    }

    #[test]
    fn ref_view_unaffected_by_branch_movement() {
        let dir = TempDir::new("ref-vs-branch");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(dir.path(), "file.txt", "v1");
        stage_and_commit(&repo, "v1", &["file.txt"]);

        let workspace = Workspace::open(dir.path()).unwrap();
        let pinned = head_ref(&repo);

        write_file(dir.path(), "file.txt", "v2");
        stage_and_commit(&repo, "v2", &["file.txt"]);

        // HEAD (the branch) has moved on to v2; the pinned ref must not.
        let pinned_view = workspace.at(&pinned).unwrap();
        assert_eq!(pinned_view.read(Path::new("file.txt")).unwrap(), b"v1");

        let moved_ref = head_ref(&repo);
        let head_view = workspace.at(&moved_ref).unwrap();
        assert_eq!(head_view.read(Path::new("file.txt")).unwrap(), b"v2");
    }

    #[test]
    fn ref_view_lists_directory() {
        let dir = TempDir::new("ref-list-dir");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(dir.path(), "dir/a.txt", "a");
        write_file(dir.path(), "dir/b.txt", "b");
        stage_and_commit(&repo, "initial", &["dir/a.txt", "dir/b.txt"]);

        let workspace = Workspace::open(dir.path()).unwrap();
        let r = head_ref(&repo);
        let view = workspace.at(&r).unwrap();

        let entries = view.list_dir(Path::new("dir")).unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["a.txt", "b.txt"]);
    }

    #[test]
    #[cfg(unix)]
    fn ref_view_errors_on_symlink() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new("ref-symlink");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(dir.path(), "target.txt", "target-content");
        symlink("target.txt", dir.path().join("link.txt")).unwrap();
        stage_and_commit(&repo, "initial", &["target.txt", "link.txt"]);

        let workspace = Workspace::open(dir.path()).unwrap();
        let r = head_ref(&repo);
        let view = workspace.at(&r).unwrap();

        let result = view.read(Path::new("link.txt"));
        assert!(matches!(
            result,
            Err(FsError::Unsupported {
                reason: "symlink",
                ..
            })
        ));
    }
}
