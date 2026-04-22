use std::fmt::Write;

// Re-export the canonical types and data from the library.
// The library owns the single source of truth; this module only adds
// binary-specific plain-text formatters that depend on super::write_em_dash_line.
pub use agentchrome::examples_data::{
    CommandGroupListing, CommandGroupSummary, ExampleEntry, all_examples,
};

// =============================================================================
// Output formatting (binary-only — depends on super::write_em_dash_line)
// =============================================================================

pub(super) fn format_plain_summary(groups: &[CommandGroupSummary]) -> String {
    let mut out = String::new();
    for (i, group) in groups.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        super::write_em_dash_line(&mut out, &group.command, &group.description);
        if let Some(first) = group.examples.first() {
            let _ = writeln!(out, "  {}", first.cmd);
        }
    }
    out
}

pub(super) fn format_plain_detail(group: &CommandGroupSummary) -> String {
    let mut out = String::new();
    super::write_em_dash_line(&mut out, &group.command, &group.description);
    for example in &group.examples {
        out.push('\n');
        let _ = writeln!(out, "  # {}", example.description);
        let _ = writeln!(out, "  {}", example.cmd);
    }
    out
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_summary_contains_all_groups() {
        let groups = all_examples();
        let output = format_plain_summary(&groups);
        for group in &groups {
            assert!(
                output.contains(&group.command),
                "Summary missing group '{}'",
                group.command
            );
        }
    }

    #[test]
    fn plain_summary_does_not_start_with_json() {
        let groups = all_examples();
        let output = format_plain_summary(&groups);
        assert!(!output.starts_with('['));
        assert!(!output.starts_with('{'));
    }

    #[test]
    fn plain_detail_contains_descriptions_and_commands() {
        let groups = all_examples();
        let group = groups.iter().find(|g| g.command == "navigate").unwrap();
        let output = format_plain_detail(group);
        assert!(output.contains("navigate"));
        for example in &group.examples {
            assert!(
                output.contains(&example.cmd),
                "Detail missing cmd: {}",
                example.cmd
            );
            assert!(
                output.contains(&example.description),
                "Detail missing description: {}",
                example.description
            );
        }
    }
}
