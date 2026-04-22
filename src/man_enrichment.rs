use std::borrow::Cow;

use crate::capabilities::CapabilitiesManifest;
use crate::examples_data::CommandGroupSummary;

/// Render a man page with enrichment sections appended.
///
/// Centralises the render-then-enrich sequence used by both `cargo xtask man`
/// (file generation) and `agentchrome man` (runtime stdout) so the two paths
/// cannot drift — notably, both use `Man::date("")` for byte-determinism.
pub fn render_enriched(
    cmd: clap::Command,
    short_name: &str,
    manifest: &CapabilitiesManifest,
    examples: &[CommandGroupSummary],
) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    clap_mangen::Man::new(cmd).date("").render(&mut buf)?;
    let enrichment = enrich_for(short_name, manifest, examples);
    if !enrichment.is_empty() {
        buf.extend_from_slice(enrichment.as_bytes());
    }
    Ok(buf)
}

/// Emit roff-formatted CAPABILITIES and EXAMPLES sections for the named command.
///
/// Returns an empty string when `cmd_name` matches nothing in either source,
/// so top-level and leaf subcommands without enrichment data pass through cleanly.
/// Iterates source `Vec`s in declared order — no hash-map walks.
#[must_use]
pub fn enrich_for(
    cmd_name: &str,
    manifest: &CapabilitiesManifest,
    examples: &[CommandGroupSummary],
) -> String {
    let mut out = String::new();

    if let Some(descriptor) = manifest.commands.iter().find(|c| c.name == cmd_name) {
        out.push_str(".SH CAPABILITIES\n");
        out.push_str(".PP\n");
        out.push_str(&escape_roff(&descriptor.description));
        out.push('\n');

        if let Some(subs) = &descriptor.subcommands {
            for sub in subs {
                out.push_str(".TP\n");
                out.push_str(".B ");
                out.push_str(&escape_roff(&sub.name));
                out.push('\n');
                out.push_str(&escape_roff(&sub.description));
                out.push('\n');

                if let Some(args) = &sub.args {
                    for arg in args {
                        out.push_str(".TP\n");
                        out.push_str(".B ");
                        out.push_str(&escape_roff(&arg.name));
                        out.push('\n');
                        if !arg.description.is_empty() {
                            out.push_str(&escape_roff(&arg.description));
                            out.push('\n');
                        }
                    }
                }

                if let Some(flags) = &sub.flags {
                    for flag in flags {
                        out.push_str(".TP\n");
                        out.push_str(".B ");
                        out.push_str(&escape_roff(&flag.name));
                        out.push('\n');
                        if !flag.description.is_empty() {
                            out.push_str(&escape_roff(&flag.description));
                            out.push('\n');
                        }
                    }
                }
            }
        }
    }

    if let Some(group) = examples.iter().find(|g| g.command == cmd_name) {
        out.push_str(".SH EXAMPLES\n");
        out.push_str(".PP\n");
        out.push_str("Examples:\n");
        for entry in &group.examples {
            out.push_str(".TP\n");
            out.push_str(".B \\`");
            out.push_str(&escape_roff(&entry.cmd));
            out.push_str("\\`\n");
            out.push_str(&escape_roff(&entry.description));
            out.push('\n');
        }
    }

    out
}

/// Escape roff special characters that appear at the start of a line.
///
/// A leading `.` or `'` triggers roff macro interpretation. Prefix such lines
/// with `\&` to suppress it without affecting rendered output.
fn escape_roff(s: &str) -> Cow<'_, str> {
    let needs_escape = s
        .lines()
        .any(|line| line.starts_with('.') || line.starts_with('\''));
    if !needs_escape {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 8);
    for (i, line) in s.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if line.starts_with('.') || line.starts_with('\'') {
            out.push_str("\\&");
        }
        out.push_str(line);
    }
    Cow::Owned(out)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::{CommandDescriptor, SubcommandDescriptor};
    use crate::examples_data::ExampleEntry;

    fn make_manifest_with(name: &str) -> CapabilitiesManifest {
        CapabilitiesManifest {
            name: "agentchrome".into(),
            version: "1.0.0".into(),
            commands: vec![CommandDescriptor {
                name: name.into(),
                description: format!("{name} does things"),
                subcommands: Some(vec![SubcommandDescriptor {
                    name: format!("{name} list"),
                    description: "List items".into(),
                    args: None,
                    flags: None,
                }]),
                session_file: None,
            }],
            global_flags: None,
            exit_codes: None,
        }
    }

    fn make_examples_with(name: &str) -> Vec<CommandGroupSummary> {
        vec![CommandGroupSummary {
            command: name.into(),
            description: format!("{name} examples"),
            examples: vec![ExampleEntry {
                cmd: format!("agentchrome {name} list"),
                description: "List all items".into(),
                flags: None,
            }],
        }]
    }

    #[test]
    fn command_present_in_both_sources_emits_both_sections() {
        let manifest = make_manifest_with("tabs");
        let examples = make_examples_with("tabs");
        let output = enrich_for("tabs", &manifest, &examples);
        assert!(output.contains(".SH CAPABILITIES"), "missing CAPABILITIES");
        assert!(output.contains(".SH EXAMPLES"), "missing EXAMPLES");
        assert!(output.contains("tabs does things"));
        assert!(output.contains("agentchrome tabs list"));
    }

    #[test]
    fn command_present_only_in_examples_emits_only_examples() {
        let manifest = make_manifest_with("navigate");
        let examples = make_examples_with("tabs");
        let output = enrich_for("tabs", &manifest, &examples);
        assert!(
            !output.contains(".SH CAPABILITIES"),
            "unexpected CAPABILITIES"
        );
        assert!(output.contains(".SH EXAMPLES"), "missing EXAMPLES");
    }

    #[test]
    fn command_present_only_in_capabilities_emits_only_capabilities() {
        let manifest = make_manifest_with("tabs");
        let examples = make_examples_with("navigate");
        let output = enrich_for("tabs", &manifest, &examples);
        assert!(output.contains(".SH CAPABILITIES"), "missing CAPABILITIES");
        assert!(!output.contains(".SH EXAMPLES"), "unexpected EXAMPLES");
    }

    #[test]
    fn command_absent_from_both_returns_empty_string() {
        let manifest = make_manifest_with("navigate");
        let examples = make_examples_with("navigate");
        let output = enrich_for("tabs", &manifest, &examples);
        assert!(output.is_empty(), "expected empty string, got: {output}");
    }
}
