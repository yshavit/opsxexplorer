## Context

The `filesystem` capability already provides a read-only view rooted at either the live disk or a resolved git commit's tree (`vfs::Workspace` / `vfs::Fs` / `vfs::GitRef`). See proposal.md for why a change model is needed now. This design covers how that model is shaped on top of `vfs`.

## Goals / Non-Goals

**Goals:**
- Represent the set of active and archived changes.
- Resolve, per change, the correct diff base as defined in `specs/change-model/spec.md` (live view for active, pinned pre-archive commit for archived).
- Keep resolved-change data free of borrowed lifetimes so it can be held across UI frames without fighting the borrow checker.

**Non-Goals:**
- Rendering this model in ratatui — a separate follow-up change.
- Computing or rendering the actual per-requirement diff content — this change only determines *which* spec-of-record state to diff against, not the diff itself.
- Supporting a workspace-independent-of-changes use case (e.g. browsing spec history starting from the spec of record rather than from a change) — noted as a deliberate non-goal for now, see Risks below.

## Decisions

**`Change` is a newtype around a change's path relative to `openspec/changes/` (`Change(String)`), not a struct with separate fields.** For an active change this is just its name (e.g. `change-modeling`); for an archived one it naturally includes the `archive/` segment (e.g. `archive/2026-08-07-add-readonly-filesystem`), since that's exactly its path under `openspec/changes/`. This makes path derivation uniform: the on-disk path is always `root/openspec/changes/{value}`, whether active or archived — no branching on status. For archived changes, the display name and archive date are derived on demand by stripping the `archive/` and `${date}-` segments from the value. `Changes` still keeps `active`/`archived` as separate `Vec<Change>` fields for convenient list rendering, even though status is technically re-derivable from whether the value starts with `archive/` — an intentional, harmless duplication traded for not re-parsing the value every time either list is rendered. Alternative considered: an enum mirroring `vfs::Fs`'s `Disk`/`Git` split (`Change::Active(..) | Change::Archived(..)`). Rejected: `Change` has no per-status behavior divergence at all, so an enum would add pure ceremony.

**Diff-base resolution is reified into a `DiffBase` enum and paired with its `Change` in a `ChangeView`, resolved lazily.** `DiffBase` is `Current` (active) or `At(GitRef)` (archived), mirroring `Fs::Disk` / `Fs::Git`. Resolution happens only when a change is actually opened, not eagerly for every discovered change, because resolving an archived change's base requires walking git history to find the commit before its archiving commit — a cost not worth paying for changes the user never looks at. `ChangeView` owns a `GitRef` (a plain `Copy` `Oid` wrapper, see `vfs::GitRef`), not a live `Fs`, so it carries no lifetime tied to `Workspace` and can be stored and passed around freely (e.g. across ratatui redraws) without borrow-checker friction.

**An active change's diff base is the live disk view (`Workspace::current()`), not a pinned HEAD commit.** This reflects uncommitted edits to spec of record files as the user makes them, consistent with "active" meaning "still being worked on."

**The archived-change anchor commit is the earliest commit that introduced any file under the change's archived directory — not "the commit that archived it."** Real history is messy: a change's directory under `archive/` can be touched by more than one commit (files added across a few commits, corrections made shortly after archiving), so there's often no single well-defined "archiving commit" to walk back from. Anchoring to the earliest commit that ever introduced a file at that path is simple and well-defined: it gets the right answer when archiving was one atomic commit, and a reasonable, conservative answer when it wasn't (pinned to before the directory existed at all). Documented as a deliberate simplification, not a claim that it always identifies one specific "archiving event."

**`Changes` owns the `vfs::Workspace` it discovers from**, rather than the workspace living as a sibling field alongside `Changes` on some outer app struct. Every consumer of `Changes` in the current diff-first use case also needs the workspace to resolve or read a change's content, and discovery already requires opening a workspace, so the coupling exists at construction regardless of whether it's retained afterward. Alternative considered: keep `Workspace` as a sibling field one level up, anticipating a possible future where the spec of record is browsed independent of any change (e.g. per-spec history). Deferred as YAGNI: because `ChangeView`/`GitRef` are lifetime-free, moving `Workspace`'s ownership later is a small, compiler-guided refactor (move the field, add a parameter to the handful of methods that used it, fix call sites) rather than a structural rewrite touching stored lifetimes.

**Naming:** the top-level struct is `Changes`, not `Workspace` — `vfs::Workspace` already uses that name for the git-backed filesystem root, and the two are different concepts.

## Risks / Trade-offs

- [Risk] A change's archived directory can be touched by more than one commit, so there's no single unambiguous "the archiving commit" in general. → [Mitigation] Defined as a simplification: use the earliest commit that introduced any file under that path (see Decisions). Resolve lazily, only for a change the user actually opens; treat resolution failure as a distinct, surfaced error rather than silently falling back to HEAD or the live view.
- [Risk] Bundling `Workspace` inside `Changes` creates light coupling that would need undoing if a workspace-independent-of-changes feature (e.g. spec-history browsing) is built later. → [Mitigation] Already established that `ChangeView`/`GitRef` hold no borrowed lifetime, so this is a mechanical, compiler-guided refactor rather than a structural one, when and if that future materializes.
