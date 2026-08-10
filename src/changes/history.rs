use std::collections::HashMap;
use std::path::PathBuf;

use git2::{Repository, Sort};

use crate::vfs::GitRef;

use super::Change;

/// What history resolved for one archived change: the timestamp of the
/// commit that first introduced its directory (used as a sort tiebreaker),
/// and that commit's first parent as the diff base. `diff_base` is `None`
/// when the introducing commit is a root commit with no parent — a case
/// distinct from the change being absent from the map entirely (no
/// introducing commit found at all).
pub struct Resolution {
    pub time: i64,
    pub diff_base: Option<GitRef>,
}

/// Resolves every archived change's introducing commit in a single
/// traversal of history (reachable from HEAD, oldest first): each
/// still-unresolved change is probed at every commit, and its *first*
/// sighting — the first commit whose tree contains the change's directory —
/// is recorded and the change dropped from the working set. The traversal
/// ends early once every change is resolved.
///
/// A change absent from the returned map has no resolvable introducing
/// commit: its directory was never committed, or a revwalk/git error ended
/// the traversal before it was reached. On such an error, changes already
/// resolved are kept; only the remainder go unresolved.
pub fn resolve_all(repo: &Repository, changes: &[Change]) -> HashMap<Change, Resolution> {
    let mut resolved = HashMap::new();
    let mut pending: Vec<(&Change, PathBuf)> = changes
        .iter()
        .map(|change| (change, change.relative_path()))
        .collect();

    let revwalk = (|| -> Result<_, git2::Error> {
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME | Sort::REVERSE)?;
        Ok(revwalk)
    })();
    let Ok(revwalk) = revwalk else {
        return resolved;
    };

    for oid in revwalk {
        if pending.is_empty() {
            break;
        }
        let Ok(oid) = oid else { break };
        let Ok(commit) = repo.find_commit(oid) else {
            break;
        };
        let Ok(tree) = commit.tree() else { break };

        pending.retain(|(change, path)| {
            if tree.get_path(path).is_err() {
                return true;
            }
            let diff_base = commit.parent_id(0).ok().map(|oid| GitRef { oid });
            resolved.insert(
                (*change).clone(),
                Resolution {
                    time: commit.time().seconds(),
                    diff_base,
                },
            );
            false
        });
    }

    resolved
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;
    use git2::Repository;

    #[test]
    fn resolve_all_records_each_change_at_its_own_introducing_commit() {
        let dir = TempDir::new("history-resolve-all-multi");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        let spec_v1 =
            stage_and_commit_at(&repo, "spec v1", &["openspec/specs/cap/spec.md"], 1_000_000);

        // Introduce "aaa", then an unrelated commit, then "bbb" - interleaved,
        // so a batched traversal must not conflate the two changes. Explicit
        // timestamps avoid same-second ties under `Sort::TIME`.
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-aaa/proposal.md",
            "x",
        );
        let aaa_base = spec_v1;
        stage_and_commit_at(
            &repo,
            "archive aaa",
            &["openspec/changes/archive/2026-01-01-aaa/proposal.md"],
            1_000_100,
        );

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v2-unrelated");
        let unrelated = stage_and_commit_at(
            &repo,
            "unrelated",
            &["openspec/specs/cap/spec.md"],
            1_000_200,
        );

        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-bbb/proposal.md",
            "y",
        );
        let bbb_base = unrelated;
        stage_and_commit_at(
            &repo,
            "archive bbb",
            &["openspec/changes/archive/2026-01-01-bbb/proposal.md"],
            1_000_300,
        );

        let aaa = Change("archive/2026-01-01-aaa".to_string());
        let bbb = Change("archive/2026-01-01-bbb".to_string());
        let resolved = resolve_all(&repo, &[aaa.clone(), bbb.clone()]);

        assert_eq!(
            resolved.get(&aaa).unwrap().diff_base,
            Some(GitRef { oid: aaa_base })
        );
        assert_eq!(
            resolved.get(&bbb).unwrap().diff_base,
            Some(GitRef { oid: bbb_base })
        );
    }

    #[test]
    fn resolve_all_uses_first_sighting_not_a_later_touch() {
        let dir = TempDir::new("history-resolve-all-first-sighting");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(&repo, "spec v1", &["openspec/specs/cap/spec.md"]);

        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "x",
        );
        stage_and_commit(
            &repo,
            "archive old-thing",
            &["openspec/changes/archive/2026-01-01-old-thing/proposal.md"],
        );
        let introducing_time = repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .time()
            .seconds();

        // A later commit also touches the same directory; it must not shift the timestamp.
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "x-corrected",
        );
        stage_and_commit(
            &repo,
            "fix typo",
            &["openspec/changes/archive/2026-01-01-old-thing/proposal.md"],
        );

        let change = Change("archive/2026-01-01-old-thing".to_string());
        let resolved = resolve_all(&repo, std::slice::from_ref(&change));
        assert_eq!(resolved.get(&change).unwrap().time, introducing_time);
    }

    #[test]
    fn resolve_all_omits_never_committed_change() {
        let dir = TempDir::new("history-resolve-all-uncommitted");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(&repo, "spec v1", &["openspec/specs/cap/spec.md"]);

        let change = Change("archive/2026-01-01-never-committed".to_string());
        let resolved = resolve_all(&repo, std::slice::from_ref(&change));
        assert!(!resolved.contains_key(&change));
    }

    #[test]
    fn resolve_all_distinguishes_root_commit_from_unresolvable() {
        let dir = TempDir::new("history-resolve-all-root");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "x",
        );
        stage_and_commit(
            &repo,
            "root commit introduces change",
            &["openspec/changes/archive/2026-01-01-old-thing/proposal.md"],
        );
        let root_time = repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .time()
            .seconds();

        let change = Change("archive/2026-01-01-old-thing".to_string());
        let resolved = resolve_all(&repo, std::slice::from_ref(&change));

        // Present in the map (unlike a never-committed change) with a
        // timestamp, but no diff base: the root commit has no parent.
        let resolution = resolved.get(&change).unwrap();
        assert_eq!(resolution.time, root_time);
        assert_eq!(resolution.diff_base, None);
    }
}
