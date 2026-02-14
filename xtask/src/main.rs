use std::fs;
use std::path::Path;

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

    let cmd = chrome_cli::command();
    let mut count = 0;

    // Generate top-level man page
    render_man_page(&cmd, "chrome-cli", out_dir);
    count += 1;

    // Generate man pages for all subcommands (recursively)
    count += generate_subcommand_pages(&cmd, "chrome-cli", out_dir);

    println!("Generated {count} man pages in {}", out_dir.display());
}

fn generate_subcommand_pages(cmd: &clap::Command, prefix: &str, out_dir: &Path) -> usize {
    let mut count = 0;
    for sub in cmd.get_subcommands() {
        if sub.get_name() == "help" {
            continue;
        }
        let page_name = format!("{prefix}-{}", sub.get_name());
        render_man_page(sub, &page_name, out_dir);
        count += 1;

        // Recurse into nested subcommands
        count += generate_subcommand_pages(sub, &page_name, out_dir);
    }
    count
}

fn render_man_page(cmd: &clap::Command, name: &str, out_dir: &Path) {
    let path = out_dir.join(format!("{name}.1"));
    let man = clap_mangen::Man::new(cmd.clone());
    let mut buf = Vec::new();
    man.render(&mut buf)
        .unwrap_or_else(|e| panic!("failed to render man page for {name}: {e}"));
    fs::write(&path, buf).unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
    println!("  {}", path.display());
}
