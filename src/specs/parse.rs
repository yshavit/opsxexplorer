//! The only file that mentions `mdq` — see
//! `2026-08-08-spec-model/design.md`'s "module stays plain `String`s"
//! decision. Parses OpenSpec spec markdown (main and delta) using
//! `mdq::md_elem`'s hierarchical section tree: a `##` section's body
//! contains its `###` subsections, whose bodies contain their `####`
//! subsections, which is exactly the Requirements -> Requirement -> Scenario
//! shape.

use std::path::Path;

use mdq::md_elem::elem::{Image, Inline, Link, List, Section, Span, Text};
use mdq::md_elem::{MdContext, MdDoc, MdElem, ParseOptions};
use mdq::output::{MdWriter, MdWriterOptions};

use crate::specs::error::{Location, SpecError, StructureErrorKind};
use crate::specs::model::{
    Delta, DeltaEntry, DeltaOp, Rename, Requirement, Scenario, Spec, UnrecognizedSection,
};

pub(crate) fn parse_spec(path: &Path, text: &str) -> Result<Spec, SpecError> {
    let doc = parse_doc(path, text)?;

    let mut purpose = None;
    let mut requirements = Vec::new();
    let mut unrecognized_sections = Vec::new();
    for section in depth2_sections(&doc.roots) {
        let title = heading_text(&section.title);
        match title.as_str() {
            "Purpose" => purpose = Some(render_body(&doc.ctx, &section.body)),
            "Requirements" => {
                requirements = parse_requirements_section(path, &doc.ctx, &title, &section.body)?;
            }
            other => unrecognized_sections.push(UnrecognizedSection {
                title: other.to_string(),
                body: render_body(&doc.ctx, &section.body),
            }),
        }
    }
    Ok(Spec {
        purpose,
        requirements,
        unrecognized_sections,
    })
}

pub(crate) fn parse_delta(path: &Path, text: &str) -> Result<Delta, SpecError> {
    let doc = parse_doc(path, text)?;

    let mut purpose = None;
    let mut entries = Vec::new();
    let mut renames = Vec::new();
    let mut unrecognized_sections = Vec::new();
    for section in depth2_sections(&doc.roots) {
        let title = heading_text(&section.title);
        match title.as_str() {
            "Purpose" => purpose = Some(render_body(&doc.ctx, &section.body)),
            "ADDED Requirements" => {
                entries.extend(parse_delta_entries_section(
                    path,
                    &doc.ctx,
                    &title,
                    &section.body,
                    DeltaOp::Added,
                )?);
            }
            "MODIFIED Requirements" => {
                entries.extend(parse_delta_entries_section(
                    path,
                    &doc.ctx,
                    &title,
                    &section.body,
                    DeltaOp::Modified,
                )?);
            }
            "REMOVED Requirements" => {
                entries.extend(parse_delta_entries_section(
                    path,
                    &doc.ctx,
                    &title,
                    &section.body,
                    DeltaOp::Removed,
                )?);
            }
            "RENAMED Requirements" => {
                renames.extend(parse_renames(path, &title, &section.body)?);
            }
            other => unrecognized_sections.push(UnrecognizedSection {
                title: other.to_string(),
                body: render_body(&doc.ctx, &section.body),
            }),
        }
    }
    Ok(Delta {
        purpose,
        entries,
        renames,
        unrecognized_sections,
    })
}

fn parse_doc(path: &Path, text: &str) -> Result<MdDoc, SpecError> {
    MdDoc::parse(text, &ParseOptions::gfm()).map_err(|source| SpecError::Markdown {
        path: path.to_path_buf(),
        source: Box::new(source),
    })
}

fn writer_options() -> MdWriterOptions {
    MdWriterOptions {
        // Any value here re-flows prose and wrecks line-level diffs.
        text_width: None,
        // Otherwise mdq injects `-----` separators absent from every source file.
        include_thematic_breaks: false,
        ..Default::default()
    }
}

fn render_body(ctx: &MdContext, elems: &[MdElem]) -> String {
    let writer = MdWriter::with_options(writer_options());
    let mut out = String::new();
    writer.write(ctx, elems, &mut out);
    out.trim_end().to_string()
}

/// Depth-2 (`## ...`) sections, whether they sit at the top level (delta
/// specs) or nested one level inside a depth-1 `# <capability>
/// Specification` wrapper (main specs).
fn depth2_sections(elems: &[MdElem]) -> Vec<&Section> {
    let mut out = Vec::new();
    for elem in elems {
        if let MdElem::Section(s) = elem {
            match s.depth {
                1 => out.extend(depth2_sections(&s.body)),
                2 => out.push(s),
                _ => {}
            }
        }
    }
    out
}

fn parse_requirements_section(
    path: &Path,
    ctx: &MdContext,
    section_name: &str,
    body: &[MdElem],
) -> Result<Vec<Requirement>, SpecError> {
    let mut out = Vec::new();
    for elem in body {
        match elem {
            MdElem::Section(s) if s.depth == 3 => out.push(parse_requirement(ctx, s)),
            MdElem::Section(s) if s.depth == 4 => {
                return Err(scenario_before_requirement_error(path, section_name));
            }
            _ => {}
        }
    }
    Ok(out)
}

fn parse_delta_entries_section(
    path: &Path,
    ctx: &MdContext,
    section_name: &str,
    body: &[MdElem],
    op: DeltaOp,
) -> Result<Vec<DeltaEntry>, SpecError> {
    let mut out = Vec::new();
    for elem in body {
        match elem {
            MdElem::Section(s) if s.depth == 3 => {
                let requirement = parse_requirement(ctx, s);
                out.push(DeltaEntry { op, requirement });
            }
            MdElem::Section(s) if s.depth == 4 => {
                return Err(scenario_before_requirement_error(path, section_name));
            }
            _ => {}
        }
    }
    Ok(out)
}

fn parse_requirement(ctx: &MdContext, section: &Section) -> Requirement {
    let name = heading_name(&section.title, "Requirement: ");

    let split_at = section
        .body
        .iter()
        .position(|e| matches!(e, MdElem::Section(_)))
        .unwrap_or(section.body.len());
    let (intro_elems, rest) = section.body.split_at(split_at);
    let intro = render_body(ctx, intro_elems);

    let mut scenarios = Vec::new();
    for elem in rest {
        let MdElem::Section(s) = elem else { continue };
        let scenario_name = heading_name(&s.title, "Scenario: ");
        let body = render_body(ctx, &s.body);
        scenarios.push(Scenario {
            name: scenario_name,
            body,
        });
    }

    Requirement {
        name,
        intro,
        scenarios,
    }
}

fn parse_renames(
    path: &Path,
    section_name: &str,
    body: &[MdElem],
) -> Result<Vec<Rename>, SpecError> {
    let mut renames = Vec::new();
    let mut pending_from: Option<String> = None;

    for elem in body {
        let MdElem::List(List { items, .. }) = elem else {
            continue;
        };
        for item in items {
            let text = list_item_text(&item.item);
            let text = text.trim();
            if let Some(rest) = text.strip_prefix("FROM:") {
                if pending_from.is_some() {
                    return Err(rename_missing_target_error(path, section_name));
                }
                pending_from = Some(requirement_name_from_bullet(rest.trim()));
            } else if let Some(rest) = text.strip_prefix("TO:") {
                let Some(from) = pending_from.take() else {
                    return Err(rename_missing_target_error(path, section_name));
                };
                renames.push(Rename {
                    from,
                    to: requirement_name_from_bullet(rest.trim()),
                });
            }
        }
    }

    if pending_from.is_some() {
        return Err(rename_missing_target_error(path, section_name));
    }

    Ok(renames)
}

/// Strips the backticked `` `### Requirement: <name>` `` wrapper the OpenSpec
/// convention uses inside a `- FROM:`/`- TO:` bullet, once plain-text
/// flattening has already stripped the backticks themselves.
fn requirement_name_from_bullet(text: &str) -> String {
    text.strip_prefix("### Requirement: ")
        .unwrap_or(text)
        .trim()
        .to_string()
}

fn list_item_text(elems: &[MdElem]) -> String {
    let mut out = String::new();
    for elem in elems {
        if let MdElem::Paragraph(p) = elem {
            flatten_inlines_into(&p.body, &mut out);
        }
    }
    out
}

fn heading_text(title: &[Inline]) -> String {
    flatten_inlines(title).trim().to_string()
}

fn heading_name(title: &[Inline], prefix: &str) -> String {
    let text = flatten_inlines(title);
    text.strip_prefix(prefix)
        .unwrap_or(&text)
        .trim()
        .to_string()
}

fn flatten_inlines(inlines: &[Inline]) -> String {
    let mut out = String::new();
    flatten_inlines_into(inlines, &mut out);
    out
}

fn flatten_inlines_into(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text(Text { value, .. }) => out.push_str(value),
            Inline::Span(Span { children, .. }) => flatten_inlines_into(children, out),
            Inline::Link(Link::Standard(link)) => flatten_inlines_into(&link.display, out),
            Inline::Link(Link::Autolink(link)) => out.push_str(&link.url),
            Inline::Image(Image { alt, .. }) => out.push_str(alt),
            Inline::Footnote(fid) => {
                out.push_str("[^");
                out.push_str(fid.as_str());
                out.push(']');
            }
        }
    }
}

fn scenario_before_requirement_error(path: &Path, section: &str) -> SpecError {
    SpecError::Structure {
        path: path.to_path_buf(),
        at: Location {
            section: section.to_string(),
            requirement: None,
        },
        kind: StructureErrorKind::ScenarioBeforeRequirement,
    }
}

fn rename_missing_target_error(path: &Path, section: &str) -> SpecError {
    SpecError::Structure {
        path: path.to_path_buf(),
        at: Location {
            section: section.to_string(),
            requirement: None,
        },
        kind: StructureErrorKind::RenameMissingTarget,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specs::error::StructureErrorKind;
    use crate::specs::model::DeltaOp;
    use std::path::Path;

    fn req_names(reqs: &[Requirement]) -> Vec<&str> {
        reqs.iter().map(|r| r.name.as_str()).collect()
    }

    fn entry_names(entries: &[DeltaEntry]) -> Vec<&str> {
        entries
            .iter()
            .map(|e| e.requirement.name.as_str())
            .collect()
    }

    fn section_titles(sections: &[UnrecognizedSection]) -> Vec<&str> {
        sections.iter().map(|s| s.title.as_str()).collect()
    }

    // --- 4.7: real files ---

    #[test]
    fn parses_change_model_main_spec() {
        let spec = parse_spec(
            Path::new("openspec/specs/change-model/spec.md"),
            &std::fs::read_to_string("openspec/specs/change-model/spec.md").unwrap(),
        )
        .unwrap();
        assert!(spec.purpose.as_deref().is_some_and(|p| !p.is_empty()));
        assert_eq!(
            req_names(&spec.requirements),
            vec![
                "Change discovery",
                "Diff base for an active change is the live spec of record",
                "Diff base for an archived change is the commit before the directory first appears",
                "Resolved diff base travels with its change",
                "Both sides of a change's diff are reachable from the resolved change",
                "Diff bases for archived changes are resolved during discovery",
                "Archived change resolution reflects history as of discovery",
            ]
        );
        let total_scenarios: usize = spec.requirements.iter().map(|r| r.scenarios.len()).sum();
        assert_eq!(total_scenarios, 18);
    }

    #[test]
    fn parses_filesystem_main_spec() {
        let spec = parse_spec(
            Path::new("openspec/specs/filesystem/spec.md"),
            &std::fs::read_to_string("openspec/specs/filesystem/spec.md").unwrap(),
        )
        .unwrap();
        assert_eq!(spec.requirements.len(), 5);
        let total_scenarios: usize = spec.requirements.iter().map(|r| r.scenarios.len()).sum();
        assert_eq!(total_scenarios, 10);
    }

    #[test]
    fn parses_tui_main_spec() {
        let spec = parse_spec(
            Path::new("openspec/specs/tui/spec.md"),
            &std::fs::read_to_string("openspec/specs/tui/spec.md").unwrap(),
        )
        .unwrap();
        assert_eq!(
            req_names(&spec.requirements),
            vec![
                "Two-pane layout",
                "Focus moves between the two panes",
                "q exits the application",
                "Terminal state is restored on exit",
            ]
        );
        let total_scenarios: usize = spec.requirements.iter().map(|r| r.scenarios.len()).sum();
        assert_eq!(total_scenarios, 9);
    }

    #[test]
    fn parses_tui_changelist_main_spec() {
        let spec = parse_spec(
            Path::new("openspec/specs/tui-changelist/spec.md"),
            &std::fs::read_to_string("openspec/specs/tui-changelist/spec.md").unwrap(),
        )
        .unwrap();
        assert_eq!(
            req_names(&spec.requirements),
            vec![
                "Active changes listed first, alphabetically",
                "Archived section is a collapsible archived/ row",
                "Archived changes sorted alphabetically, displayed with date",
                "Single cursor navigable over active, archived-header, and archived rows",
                "Archived section toggles when its header is selected",
                "Selecting a change moves focus to the right pane",
                "Collapsing returns the cursor to the archived header",
                "Empty sections show placeholder text",
                "Left pane width is capped to its content",
                "Left pane scrolls horizontally as a single unit",
                "Horizontal scroll position is indicated with a scrollbar",
                "Horizontal scroll offset persists across selection and section toggling, clamped to all rows",
            ]
        );
        let total_scenarios: usize = spec.requirements.iter().map(|r| r.scenarios.len()).sum();
        assert_eq!(total_scenarios, 47);
    }

    #[test]
    fn parses_add_readonly_filesystem_delta() {
        let path =
            "openspec/changes/archive/2026-08-07-add-readonly-filesystem/specs/filesystem/spec.md";
        let delta = parse_delta(Path::new(path), &std::fs::read_to_string(path).unwrap()).unwrap();
        assert!(delta.purpose.as_deref().is_some_and(|p| !p.is_empty()));
        assert_eq!(delta.entries.len(), 5);
        assert!(delta.entries.iter().all(|e| e.op == DeltaOp::Added));
        assert!(delta.renames.is_empty());
        let total_scenarios: usize = delta
            .entries
            .iter()
            .map(|e| e.requirement.scenarios.len())
            .sum();
        assert_eq!(total_scenarios, 10);
    }

    #[test]
    fn parses_change_modeling_delta() {
        let path = "openspec/changes/archive/2026-08-07-change-modeling/specs/change-model/spec.md";
        let delta = parse_delta(Path::new(path), &std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(delta.entries.len(), 4);
        assert!(delta.entries.iter().all(|e| e.op == DeltaOp::Added));
        let total_scenarios: usize = delta
            .entries
            .iter()
            .map(|e| e.requirement.scenarios.len())
            .sum();
        assert_eq!(total_scenarios, 7);
    }

    #[test]
    fn parses_tui_initial_delta_for_tui() {
        let path = "openspec/changes/archive/2026-08-07-tui-initial/specs/tui/spec.md";
        let delta = parse_delta(Path::new(path), &std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(delta.entries.len(), 5);
        assert!(delta.entries.iter().all(|e| e.op == DeltaOp::Added));
    }

    #[test]
    fn parses_tui_initial_delta_for_tui_changelist() {
        let path = "openspec/changes/archive/2026-08-07-tui-initial/specs/tui-changelist/spec.md";
        let delta = parse_delta(Path::new(path), &std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(delta.entries.len(), 7);
        assert!(delta.entries.iter().all(|e| e.op == DeltaOp::Added));
        let total_scenarios: usize = delta
            .entries
            .iter()
            .map(|e| e.requirement.scenarios.len())
            .sum();
        assert_eq!(total_scenarios, 18);
    }

    #[test]
    fn parses_horizontal_scrolling_delta_added_and_modified_no_purpose() {
        let path = "openspec/changes/archive/2026-08-08-tui-changelist-horizontal-scrolling/specs/tui-changelist/spec.md";
        let delta = parse_delta(Path::new(path), &std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(delta.purpose, None);
        assert_eq!(delta.entries.len(), 4);
        let added: Vec<_> = delta
            .entries
            .iter()
            .filter(|e| e.op == DeltaOp::Added)
            .collect();
        let modified: Vec<_> = delta
            .entries
            .iter()
            .filter(|e| e.op == DeltaOp::Modified)
            .collect();
        assert_eq!(added.len(), 3);
        assert_eq!(modified.len(), 1);
        assert_eq!(
            modified[0].requirement.name,
            "Archived changes are grouped under a collapsible section"
        );
        let added_scenarios: usize = added.iter().map(|e| e.requirement.scenarios.len()).sum();
        let modified_scenarios: usize =
            modified.iter().map(|e| e.requirement.scenarios.len()).sum();
        assert_eq!(added_scenarios, 13);
        assert_eq!(modified_scenarios, 6);
    }

    // --- 4.8: synthetic parser tests ---

    #[test]
    fn removed_entry_is_heading_only() {
        let text = "## REMOVED Requirements\n\n### Requirement: Some Old Thing\n";
        let delta = parse_delta(Path::new("synthetic"), text).unwrap();
        assert_eq!(delta.entries.len(), 1);
        assert_eq!(delta.entries[0].op, DeltaOp::Removed);
        assert_eq!(delta.entries[0].requirement.name, "Some Old Thing");
        assert_eq!(delta.entries[0].requirement.intro, "");
        assert!(delta.entries[0].requirement.scenarios.is_empty());
    }

    #[test]
    fn removed_entry_with_reason_and_migration_body_parses_into_intro() {
        let text = "## REMOVED Requirements\n\n\
            ### Requirement: Some Old Thing\n\
            **Reason**: no longer needed.\n\
            **Migration**: use the new thing instead.\n";
        let delta = parse_delta(Path::new("synthetic"), text).unwrap();
        assert_eq!(delta.entries.len(), 1);
        assert_eq!(delta.entries[0].op, DeltaOp::Removed);
        assert_eq!(delta.entries[0].requirement.name, "Some Old Thing");
        assert!(
            delta.entries[0]
                .requirement
                .intro
                .contains("**Reason**: no longer needed.")
        );
        assert!(
            delta.entries[0]
                .requirement
                .intro
                .contains("**Migration**: use the new thing instead.")
        );
        assert!(delta.entries[0].requirement.scenarios.is_empty());
    }

    #[test]
    fn renamed_section_pairs_from_and_to() {
        let text = "## RENAMED Requirements\n\n\
            - FROM: `### Requirement: Old Name`\n\
            - TO: `### Requirement: New Name`\n\
            - FROM: `### Requirement: Another Old`\n\
            - TO: `### Requirement: Another New`\n";
        let delta = parse_delta(Path::new("synthetic"), text).unwrap();
        assert_eq!(
            delta.renames,
            vec![
                Rename {
                    from: "Old Name".to_string(),
                    to: "New Name".to_string(),
                },
                Rename {
                    from: "Another Old".to_string(),
                    to: "Another New".to_string(),
                },
            ]
        );
    }

    #[test]
    fn scenario_before_any_requirement_is_reported() {
        let text = "## Requirements\n\n#### Scenario: orphan\n- **WHEN** x\n- **THEN** y\n";
        let err = parse_spec(Path::new("synthetic"), text).unwrap_err();
        assert!(matches!(
            err,
            SpecError::Structure {
                kind: StructureErrorKind::ScenarioBeforeRequirement,
                ..
            }
        ));
    }

    #[test]
    fn unrecognised_delta_section_among_well_formed_ones_is_collected() {
        let text = "## ADDED Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            ## BOGUS Requirements\n\nsome text\n";
        let delta = parse_delta(Path::new("synthetic"), text).unwrap();
        assert_eq!(entry_names(&delta.entries), vec!["Foo"]);
        assert_eq!(
            delta.unrecognized_sections,
            vec![UnrecognizedSection {
                title: "BOGUS Requirements".to_string(),
                body: "some text".to_string(),
            }]
        );
    }

    #[test]
    fn several_unrecognised_delta_sections_preserve_document_order() {
        let text = "## FIRST BOGUS\n\ntext\n\n\
            ## ADDED Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            ## SECOND BOGUS\n\ntext\n\n\
            ## THIRD BOGUS\n\ntext\n";
        let delta = parse_delta(Path::new("synthetic"), text).unwrap();
        assert_eq!(
            section_titles(&delta.unrecognized_sections),
            vec!["FIRST BOGUS", "SECOND BOGUS", "THIRD BOGUS"]
        );
    }

    #[test]
    fn no_unrecognised_delta_sections_yields_empty_vec() {
        let text = "## ADDED Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n";
        let delta = parse_delta(Path::new("synthetic"), text).unwrap();
        assert!(delta.unrecognized_sections.is_empty());
    }

    #[test]
    fn unrecognised_section_in_a_spec_of_record_is_collected_and_parsing_continues() {
        let text = "## Why\n\nBecause reasons.\n\n\
            ## Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            ## BOGUS\n\nsome text\n";
        let spec = parse_spec(Path::new("synthetic"), text).unwrap();
        // The recognised sections around it still parse.
        assert_eq!(req_names(&spec.requirements), vec!["Foo"]);
        assert_eq!(
            spec.unrecognized_sections,
            vec![
                UnrecognizedSection {
                    title: "Why".to_string(),
                    body: "Because reasons.".to_string(),
                },
                UnrecognizedSection {
                    title: "BOGUS".to_string(),
                    body: "some text".to_string(),
                },
            ]
        );
    }

    #[test]
    fn no_unrecognised_sections_in_a_spec_of_record_yields_empty_vec() {
        let text = "## Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n";
        let spec = parse_spec(Path::new("synthetic"), text).unwrap();
        assert!(spec.unrecognized_sections.is_empty());
    }

    #[test]
    fn an_unrecognised_sections_body_is_the_same_rendered_body_a_requirement_intro_gets() {
        // The same rich block content on both sides: once as an unrecognised
        // section's body, once as a requirement's intro. Both go through
        // `render_body`, so the two must come back identical.
        let content = "First paragraph with `code` and **bold**.\n\n\
            - a bullet\n\
            - another bullet\n\n\
            ```text\n\
            fn example() {}\n\
            ```";

        let delta_text = format!("## BOGUS\n\n{content}\n");
        let delta = parse_delta(Path::new("synthetic"), &delta_text).unwrap();

        let spec_text = format!("## BOGUS\n\n{content}\n");
        let spec = parse_spec(Path::new("synthetic"), &spec_text).unwrap();

        let intro_text = format!("## Requirements\n\n### Requirement: Foo\n{content}\n");
        let intro_spec = parse_spec(Path::new("synthetic"), &intro_text).unwrap();
        let intro = &intro_spec.requirements[0].intro;

        assert_eq!(&delta.unrecognized_sections[0].body, intro);
        assert_eq!(&spec.unrecognized_sections[0].body, intro);
    }

    #[test]
    fn rename_missing_to_is_reported() {
        let text = "## RENAMED Requirements\n\n- FROM: `### Requirement: Old Name`\n";
        let err = parse_delta(Path::new("synthetic"), text).unwrap_err();
        assert!(matches!(
            err,
            SpecError::Structure {
                kind: StructureErrorKind::RenameMissingTarget,
                ..
            }
        ));
    }

    #[test]
    fn rename_missing_from_is_reported() {
        let text = "## RENAMED Requirements\n\n- TO: `### Requirement: New Name`\n";
        let err = parse_delta(Path::new("synthetic"), text).unwrap_err();
        assert!(matches!(
            err,
            SpecError::Structure {
                kind: StructureErrorKind::RenameMissingTarget,
                ..
            }
        ));
    }

    // --- 4.9: content-preservation ---

    #[test]
    fn body_content_survives_without_byte_equality() {
        let long_paragraph = "x".repeat(470);
        let text = format!(
            "## Requirements\n\n\
            ### Requirement: Rich content\n\
            First paragraph of the intro.\n\n\
            Second paragraph of the intro.\n\n\
            #### Scenario: has everything\n\
            - **WHEN** something happens with `inline_code` and **bold text**\n\
            - **THEN** the following applies:\n\n\
            ```text\n\
            fn example() {{\n    println!(\"hello\");\n}}\n\
            ```\n\n\
            {long_paragraph}\n"
        );
        let spec = parse_spec(Path::new("synthetic"), &text).unwrap();
        assert_eq!(spec.requirements.len(), 1);
        let req = &spec.requirements[0];

        // Multi-paragraph intro: both paragraphs present, still separated.
        assert!(req.intro.contains("First paragraph of the intro."));
        assert!(req.intro.contains("Second paragraph of the intro."));

        let scenario = &req.scenarios[0];
        assert_eq!(scenario.name, "has everything");
        // Bullets, bold, and inline code survive.
        assert!(scenario.body.contains("inline_code"));
        assert!(scenario.body.contains("bold text"));
        assert!(scenario.body.contains("- "));
        // Code block content survives.
        assert!(scenario.body.contains("fn example()"));
        assert!(scenario.body.contains("println!"));
        // The long paragraph is not broken across lines.
        assert!(scenario.body.lines().any(|l| l.len() >= 470));
    }

    // --- 4.9a: normalisation-consistency ---

    #[test]
    fn same_requirement_source_normalises_identically_on_both_sides() {
        let requirement_md = "### Requirement: Shared\n\
            Some intro text with `code` and **bold**.\n\n\
            #### Scenario: shared scenario\n\
            - **WHEN** x happens\n\
            - **THEN** y follows\n";

        let spec_text = format!("## Requirements\n\n{requirement_md}");
        let delta_text = format!("## ADDED Requirements\n\n{requirement_md}");

        let spec = parse_spec(Path::new("synthetic-spec"), &spec_text).unwrap();
        let delta = parse_delta(Path::new("synthetic-delta"), &delta_text).unwrap();

        assert_eq!(
            spec.requirements[0].intro,
            delta.entries[0].requirement.intro
        );
        assert_eq!(
            spec.requirements[0].scenarios[0].body,
            delta.entries[0].requirement.scenarios[0].body
        );
    }

    // --- 4.9b: block-structure ---

    #[test]
    fn heading_shaped_lines_inside_fence_are_not_new_requirements_or_scenarios() {
        let text = "## Requirements\n\n\
            ### Requirement: Real One\n\
            Intro text.\n\n\
            ```text\n\
            ### Requirement: Not Real\n\
            #### Scenario: Not Real\n\
            ```\n\n\
            #### Scenario: real scenario\n\
            - **WHEN** x\n\
            - **THEN** y\n";
        let spec = parse_spec(Path::new("synthetic"), text).unwrap();
        assert_eq!(req_names(&spec.requirements), vec!["Real One"]);
        assert_eq!(spec.requirements[0].scenarios.len(), 1);
        assert_eq!(spec.requirements[0].scenarios[0].name, "real scenario");
        // The fenced content is still present, just as body text, not as headings.
        assert!(
            spec.requirements[0]
                .intro
                .contains("### Requirement: Not Real")
        );
        assert!(
            spec.requirements[0]
                .intro
                .contains("#### Scenario: Not Real")
        );
    }

    // --- 4.10: scenario-subset ---

    #[test]
    fn modified_requirement_listing_fewer_scenarios_is_not_an_error() {
        let text = "## MODIFIED Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            #### Scenario: B\n\
            - **WHEN** b\n\
            - **THEN** b2\n";
        let delta = parse_delta(Path::new("synthetic"), text).unwrap();
        assert_eq!(entry_names(&delta.entries), vec!["Foo"]);
        let scenario_names: Vec<_> = delta.entries[0]
            .requirement
            .scenarios
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(scenario_names, vec!["A", "B"]);
    }
}

#[cfg(test)]
mod round_trip_spike {
    use mdq::md_elem::{MdDoc, ParseOptions};
    use mdq::output::{MdWriter, MdWriterOptions};
    use std::fs;

    fn writer_options() -> MdWriterOptions {
        MdWriterOptions {
            // Any wrapping value would re-flow prose and wreck line-level diffs.
            text_width: None,
            // Thematic breaks would inject `-----` separators absent from every source file.
            include_thematic_breaks: false,
            ..Default::default()
        }
    }

    fn render(text: &str) -> String {
        let doc = MdDoc::parse(text, &ParseOptions::gfm()).expect("valid markdown");
        let writer = MdWriter::with_options(writer_options());
        let mut out = String::new();
        writer.write(&doc.ctx, &doc.roots, &mut out);
        out
    }

    fn check_round_trip(path: &str) {
        let original = fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {path}: {e}"));

        let rendered = render(&original);

        // No content lost: every requirement/scenario heading and backticked
        // identifier from the source should still appear in the rendered output.
        for line in original.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("### Requirement:") || trimmed.starts_with("#### Scenario:") {
                assert!(
                    rendered.contains(trimmed),
                    "{path}: heading {trimmed:?} missing from rendered output:\n{rendered}"
                );
            }
        }

        // No `-----` thematic break separators injected.
        assert!(
            !rendered.lines().any(|l| l.trim() == "-----"),
            "{path}: unexpected thematic break separator in rendered output"
        );

        // The longest line in the source should come back unbroken (no re-wrapping).
        let longest_original = original.lines().map(str::len).max().unwrap_or(0);
        if longest_original > 0 {
            let longest_rendered = rendered.lines().map(str::len).max().unwrap_or(0);
            assert!(
                longest_rendered >= longest_original,
                "{path}: longest line was broken up (source max {longest_original}, rendered max {longest_rendered})"
            );
        }

        // The real acceptance criterion: the round trip is idempotent.
        let rendered_again = render(&rendered);
        assert_eq!(
            rendered, rendered_again,
            "{path}: round trip is not idempotent"
        );
    }

    #[test]
    fn round_trip_idempotent_on_tui_changelist_spec() {
        check_round_trip("openspec/specs/tui-changelist/spec.md");
    }

    #[test]
    fn round_trip_idempotent_on_horizontal_scrolling_delta() {
        check_round_trip(
            "openspec/changes/archive/2026-08-08-tui-changelist-horizontal-scrolling/specs/tui-changelist/spec.md",
        );
    }
}
