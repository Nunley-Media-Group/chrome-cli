use std::fs;
use std::path::Path;

use agentchrome::capabilities::CapabilitiesManifest;
use agentchrome::examples_data::CommandGroupSummary;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("man") => generate_man_pages(),
        Some(other) => {
            eprintln!("unknown xtask command: {other}");
            eprintln!("available commands: man");
            std::process::exit(1);
        }
        None => {
            eprintln!("usage: cargo xtask <command>");
            eprintln!("available commands: man");
            std::process::exit(1);
        }
    }
}

fn generate_man_pages() {
    let out_dir = Path::new("man");
    fs::create_dir_all(out_dir).expect("failed to create man/ directory");

    let cmd = agentchrome::command();
    let manifest = agentchrome::capabilities::build_manifest(&cmd, false);
    let examples = agentchrome::examples_data::all_examples();

    let mut count = 0;

    // Generate top-level man page
    render_man_page(&cmd, "agentchrome", out_dir, &manifest, &examples);
    count += 1;

    // Generate man pages for all subcommands (recursively)
    count += generate_subcommand_pages(&cmd, "agentchrome", out_dir, &manifest, &examples);

    println!("Generated {count} man pages in {}", out_dir.display());
}

fn generate_subcommand_pages(
    cmd: &clap::Command,
    prefix: &str,
    out_dir: &Path,
    manifest: &CapabilitiesManifest,
    examples: &[CommandGroupSummary],
) -> usize {
    let mut count = 0;
    for sub in cmd.get_subcommands() {
        if sub.get_name() == "help" {
            continue;
        }
        let page_name = format!("{prefix}-{}", sub.get_name());
        render_man_page(sub, &page_name, out_dir, manifest, examples);
        count += 1;

        // Recurse into nested subcommands
        count += generate_subcommand_pages(sub, &page_name, out_dir, manifest, examples);
    }
    count
}

fn render_man_page(
    cmd: &clap::Command,
    name: &str,
    out_dir: &Path,
    manifest: &CapabilitiesManifest,
    examples: &[CommandGroupSummary],
) {
    let path = out_dir.join(format!("{name}.1"));
    let buf =
        agentchrome::man_enrichment::render_enriched(cmd.clone(), enrichment_key(name), manifest, examples)
            .unwrap_or_else(|e| panic!("failed to render man page for {name}: {e}"));
    fs::write(&path, buf).unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
    println!("  {}", path.display());
}

/// Returns the top-level subcommand name from a man-page name, so both
/// `agentchrome-dialog` and `agentchrome-tabs-list` look up enrichment under
/// `dialog` and `tabs` respectively.
fn enrichment_key(page_name: &str) -> &str {
    page_name
        .strip_prefix("agentchrome-")
        .map_or(page_name, |rest| rest.split('-').next().unwrap_or(rest))
}
