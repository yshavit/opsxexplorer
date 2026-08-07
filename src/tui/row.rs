use crate::changes::Change;

const NO_ACTIVE_CHANGES: &str = "(no active changes)";
const NO_ARCHIVED_CHANGES: &str = "(no archived changes)";

/// A single visible row in the left pane's flattened list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row<'a> {
    Active(&'a Change),
    ArchivedHeader { expanded: bool },
    Archived(&'a Change),
    Placeholder { text: &'static str, indented: bool },
}

impl Row<'_> {
    pub fn is_selectable(&self) -> bool {
        !matches!(self, Row::Placeholder { .. })
    }
}

/// Flattens active/archived changes into the rows the left pane renders and navigates over.
pub fn flatten<'a>(
    active: &'a [Change],
    archived: &'a [Change],
    archived_expanded: bool,
) -> Vec<Row<'a>> {
    let mut rows = Vec::with_capacity(active.len() + archived.len() + 1);

    if active.is_empty() {
        rows.push(Row::Placeholder {
            text: NO_ACTIVE_CHANGES,
            indented: false,
        });
    } else {
        rows.extend(active.iter().map(Row::Active));
    }

    rows.push(Row::ArchivedHeader {
        expanded: archived_expanded,
    });

    if archived_expanded {
        if archived.is_empty() {
            rows.push(Row::Placeholder {
                text: NO_ARCHIVED_CHANGES,
                indented: true,
            });
        } else {
            rows.extend(archived.iter().map(Row::Archived));
        }
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active_changes() -> Vec<Change> {
        vec![Change("dark-mode".to_string()), Change("zebra".to_string())]
    }

    fn archived_changes() -> Vec<Change> {
        vec![
            Change("archive/2026-01-01-old-thing".to_string()),
            Change("archive/2026-06-19-bar".to_string()),
        ]
    }

    #[test]
    fn active_only_collapsed() {
        let active = active_changes();
        let archived = Vec::new();
        let rows = flatten(&active, &archived, false);
        assert_eq!(
            rows,
            vec![
                Row::Active(&active[0]),
                Row::Active(&active[1]),
                Row::ArchivedHeader { expanded: false },
            ]
        );
    }

    #[test]
    fn archived_only_expanded() {
        let active = Vec::new();
        let archived = archived_changes();
        let rows = flatten(&active, &archived, true);
        assert_eq!(
            rows,
            vec![
                Row::Placeholder {
                    text: NO_ACTIVE_CHANGES,
                    indented: false,
                },
                Row::ArchivedHeader { expanded: true },
                Row::Archived(&archived[0]),
                Row::Archived(&archived[1]),
            ]
        );
    }

    #[test]
    fn both_empty_collapsed() {
        let active = Vec::new();
        let archived = Vec::new();
        let rows = flatten(&active, &archived, false);
        assert_eq!(
            rows,
            vec![
                Row::Placeholder {
                    text: NO_ACTIVE_CHANGES,
                    indented: false,
                },
                Row::ArchivedHeader { expanded: false },
            ]
        );
    }

    #[test]
    fn both_empty_expanded() {
        let active = Vec::new();
        let archived = Vec::new();
        let rows = flatten(&active, &archived, true);
        assert_eq!(
            rows,
            vec![
                Row::Placeholder {
                    text: NO_ACTIVE_CHANGES,
                    indented: false,
                },
                Row::ArchivedHeader { expanded: true },
                Row::Placeholder {
                    text: NO_ARCHIVED_CHANGES,
                    indented: true,
                },
            ]
        );
    }

    #[test]
    fn both_populated_collapsed() {
        let active = active_changes();
        let archived = archived_changes();
        let rows = flatten(&active, &archived, false);
        assert_eq!(
            rows,
            vec![
                Row::Active(&active[0]),
                Row::Active(&active[1]),
                Row::ArchivedHeader { expanded: false },
            ]
        );
    }

    #[test]
    fn both_populated_expanded() {
        let active = active_changes();
        let archived = archived_changes();
        let rows = flatten(&active, &archived, true);
        assert_eq!(
            rows,
            vec![
                Row::Active(&active[0]),
                Row::Active(&active[1]),
                Row::ArchivedHeader { expanded: true },
                Row::Archived(&archived[0]),
                Row::Archived(&archived[1]),
            ]
        );
    }
}
