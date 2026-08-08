use std::path::Path;

use crate::changes::ChangeViews;
use crate::specs::error::SpecError;
use crate::specs::model::{DeltaOp, SpecPair};
use crate::specs::parse::{parse_delta, parse_spec};
use crate::vfs::{EntryKind, FsError};

const SPEC_FILE: &str = "spec.md";
const MAIN_SPECS_DIR: &str = "openspec/specs";

/// The capabilities a change carries delta specs for, in stable alphabetical
/// order. Reads `<change>/specs/` on the **live** view — the delta specs of
/// an archived change are read from the working tree by definition, since
/// that is the whole asymmetry [`crate::changes::Changes::views`] exists to
/// expose.
pub fn capabilities(views: &ChangeViews) -> Result<Vec<String>, FsError> {
    let dir = views.change.relative_path().join("specs");
    match views.live.list_dir(&dir) {
        Ok(entries) => {
            let mut names: Vec<String> = entries
                .into_iter()
                .filter(|e| e.kind == EntryKind::Dir)
                .map(|e| e.name)
                .collect();
            names.sort();
            Ok(names)
        }
        Err(FsError::NotFound(_)) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// Both sides of a change-and-capability pair: the delta spec, read from the
/// live working tree, and the spec of record, read at the change's resolved
/// diff base. `base` is `None` only when the delta introduces a capability
/// with no spec of record yet — a MODIFIED or REMOVED entry with no base is
/// a [`SpecError::MissingBaseSpec`], distinguishable from that tolerated case.
pub fn load(views: &ChangeViews, capability: &str) -> Result<SpecPair, SpecError> {
    let delta_path = views
        .change
        .relative_path()
        .join("specs")
        .join(capability)
        .join(SPEC_FILE);
    let delta_text = match views.live.read(&delta_path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(FsError::NotFound(_)) => {
            return Err(SpecError::MissingSpecDocument {
                capability: capability.to_string(),
            });
        }
        Err(e) => return Err(SpecError::Fs(e)),
    };
    let delta = parse_delta(&delta_path, &delta_text)?;

    let base_path = Path::new(MAIN_SPECS_DIR).join(capability).join(SPEC_FILE);
    let base = match views.base.read(&base_path) {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            Some(parse_spec(&base_path, &text)?)
        }
        Err(FsError::NotFound(_)) => None,
        Err(e) => return Err(SpecError::Fs(e)),
    };

    if base.is_none()
        && let Some(entry) = delta
            .entries
            .iter()
            .find(|e| matches!(e.op, DeltaOp::Modified | DeltaOp::Removed))
    {
        return Err(SpecError::MissingBaseSpec {
            capability: capability.to_string(),
            requirement: entry.requirement.name.clone(),
        });
    }

    Ok(SpecPair { delta, base })
}

#[cfg(test)]
mod test_support {
    use git2::{Oid, Repository, Signature};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new(label: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "opsxexplorer-specs-test-{label}-{}-{id}",
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
    use crate::changes::Changes;
    use git2::Repository;

    #[test]
    fn all_added_delta_with_no_base_loads_successfully() {
        let dir = TempDir::new("all-added-no-base");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(
            dir.path(),
            "openspec/changes/add-thing/specs/cap/spec.md",
            "## ADDED Requirements\n\n### Requirement: New\n#### Scenario: it works\n- **WHEN** a\n- **THEN** b\n",
        );
        stage_and_commit(
            &repo,
            "initial",
            &["openspec/changes/add-thing/specs/cap/spec.md"],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.active[0].clone();
        let view = changes.resolve(&change).unwrap();
        let views = changes.views(&view).unwrap();

        let pair = load(&views, "cap").unwrap();
        assert!(pair.base.is_none());
        assert_eq!(pair.delta.entries.len(), 1);
    }

    #[test]
    fn modified_entry_with_no_base_is_missing_base_spec_error() {
        let dir = TempDir::new("modified-no-base");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(
            dir.path(),
            "openspec/changes/add-thing/specs/cap/spec.md",
            "## MODIFIED Requirements\n\n### Requirement: Existing\n#### Scenario: it works\n- **WHEN** a\n- **THEN** b\n",
        );
        stage_and_commit(
            &repo,
            "initial",
            &["openspec/changes/add-thing/specs/cap/spec.md"],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.active[0].clone();
        let view = changes.resolve(&change).unwrap();
        let views = changes.views(&view).unwrap();

        let err = load(&views, "cap").unwrap_err();
        assert!(matches!(
            err,
            SpecError::MissingBaseSpec { ref capability, ref requirement }
                if capability == "cap" && requirement == "Existing"
        ));
    }

    #[test]
    fn removed_entry_with_no_base_is_missing_base_spec_error() {
        let dir = TempDir::new("removed-no-base");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(
            dir.path(),
            "openspec/changes/add-thing/specs/cap/spec.md",
            "## REMOVED Requirements\n\n### Requirement: Gone\n",
        );
        stage_and_commit(
            &repo,
            "initial",
            &["openspec/changes/add-thing/specs/cap/spec.md"],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.active[0].clone();
        let view = changes.resolve(&change).unwrap();
        let views = changes.views(&view).unwrap();

        let err = load(&views, "cap").unwrap_err();
        assert!(matches!(
            err,
            SpecError::MissingBaseSpec { ref capability, ref requirement }
                if capability == "cap" && requirement == "Gone"
        ));
        // Distinguishable from a plain missing delta document.
        assert!(!matches!(err, SpecError::MissingSpecDocument { .. }));
    }

    #[test]
    fn change_with_no_specs_dir_has_empty_capabilities_and_no_error() {
        let dir = TempDir::new("no-specs-dir");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(
            dir.path(),
            "openspec/changes/proposal-only/proposal.md",
            "x",
        );
        stage_and_commit(
            &repo,
            "initial",
            &["openspec/changes/proposal-only/proposal.md"],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.active[0].clone();
        let view = changes.resolve(&change).unwrap();
        let views = changes.views(&view).unwrap();

        assert_eq!(capabilities(&views).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn multi_capability_change_enumerates_alphabetically_and_stably() {
        let dir = TempDir::new("multi-cap");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(
            dir.path(),
            "openspec/changes/add-thing/specs/zebra/spec.md",
            "## ADDED Requirements\n\n### Requirement: Z\n",
        );
        write_file(
            dir.path(),
            "openspec/changes/add-thing/specs/alpha/spec.md",
            "## ADDED Requirements\n\n### Requirement: A\n",
        );
        stage_and_commit(
            &repo,
            "initial",
            &[
                "openspec/changes/add-thing/specs/zebra/spec.md",
                "openspec/changes/add-thing/specs/alpha/spec.md",
            ],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.active[0].clone();
        let view = changes.resolve(&change).unwrap();
        let views = changes.views(&view).unwrap();

        let expected = vec!["alpha".to_string(), "zebra".to_string()];
        assert_eq!(capabilities(&views).unwrap(), expected);
        // Repeated calls are identical.
        assert_eq!(capabilities(&views).unwrap(), expected);
    }

    #[test]
    fn capability_with_no_spec_document_fails_in_isolation() {
        let dir = TempDir::new("isolated-failure");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(
            dir.path(),
            "openspec/changes/add-thing/specs/good/spec.md",
            "## ADDED Requirements\n\n### Requirement: Fine\n",
        );
        // "bad" has a directory but no spec.md in it.
        write_file(
            dir.path(),
            "openspec/changes/add-thing/specs/bad/readme.txt",
            "not a spec",
        );
        stage_and_commit(
            &repo,
            "initial",
            &[
                "openspec/changes/add-thing/specs/good/spec.md",
                "openspec/changes/add-thing/specs/bad/readme.txt",
            ],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.active[0].clone();
        let view = changes.resolve(&change).unwrap();
        let views = changes.views(&view).unwrap();

        assert_eq!(
            capabilities(&views).unwrap(),
            vec!["bad".to_string(), "good".to_string()]
        );

        let bad_err = load(&views, "bad").unwrap_err();
        assert!(matches!(
            bad_err,
            SpecError::MissingSpecDocument { ref capability } if capability == "bad"
        ));

        // The other capability still loads successfully.
        let good = load(&views, "good").unwrap();
        assert_eq!(good.delta.entries.len(), 1);
    }

    #[test]
    fn removed_requirement_body_recoverable_from_loaded_base() {
        let dir = TempDir::new("removed-body-from-base");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(
            dir.path(),
            "openspec/specs/cap/spec.md",
            "## Requirements\n\n\
             ### Requirement: Gone\n\
             Some intro for the doomed requirement.\n\n\
             #### Scenario: still here in the base\n\
             - **WHEN** a\n\
             - **THEN** b\n",
        );
        write_file(
            dir.path(),
            "openspec/changes/remove-thing/specs/cap/spec.md",
            "## REMOVED Requirements\n\n### Requirement: Gone\n",
        );
        stage_and_commit(
            &repo,
            "initial",
            &[
                "openspec/specs/cap/spec.md",
                "openspec/changes/remove-thing/specs/cap/spec.md",
            ],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.active[0].clone();
        let view = changes.resolve(&change).unwrap();
        let views = changes.views(&view).unwrap();

        let pair = load(&views, "cap").unwrap();
        assert_eq!(pair.delta.entries[0].requirement.intro, "");
        let base = pair.base.unwrap();
        let gone = base.requirements.iter().find(|r| r.name == "Gone").unwrap();
        assert!(
            gone.intro
                .contains("Some intro for the doomed requirement.")
        );
        assert_eq!(gone.scenarios.len(), 1);
        assert_eq!(gone.scenarios[0].name, "still here in the base");
    }

    #[test]
    fn archived_change_loads_delta_from_live_and_base_from_diff_base() {
        let dir = TempDir::new("archived-load");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(
            dir.path(),
            "openspec/specs/cap/spec.md",
            "## Requirements\n\n### Requirement: Original\n#### Scenario: s\n- **WHEN** a\n- **THEN** b\n",
        );
        stage_and_commit(&repo, "spec v1", &["openspec/specs/cap/spec.md"]);

        // The archiving commit introduces the change directory (with its own
        // delta spec) and edits the spec of record in the same commit.
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/specs/cap/spec.md",
            "## ADDED Requirements\n\n### Requirement: Brand New\n",
        );
        write_file(
            dir.path(),
            "openspec/specs/cap/spec.md",
            "## Requirements\n\n### Requirement: Original Edited\n#### Scenario: s\n- **WHEN** a\n- **THEN** b\n",
        );
        stage_and_commit(
            &repo,
            "archive old-thing",
            &[
                "openspec/changes/archive/2026-01-01-old-thing/specs/cap/spec.md",
                "openspec/specs/cap/spec.md",
            ],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.archived[0].clone();
        let view = changes.resolve(&change).unwrap();
        let views = changes.views(&view).unwrap();

        let pair = load(&views, "cap").unwrap();
        assert_eq!(pair.delta.entries[0].requirement.name, "Brand New");
        let base = pair.base.unwrap();
        assert_eq!(base.requirements[0].name, "Original");
    }
}
