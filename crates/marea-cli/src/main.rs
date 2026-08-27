//! `marea new` — scaffold a marea app.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};

use marea_cli::manifest::MareaSource;
use marea_cli::options::{self, DepMode, Options};
use marea_cli::{locate, prompts, render, verify, write};

/// The tag generated manifests pin when there is no local checkout to point
/// at. Bump alongside a marea release.
const MAREA_TAG: &str = "v0.1.0";
const MAREA_GIT: &str = "https://github.com/pvg13/marea";

#[derive(Parser)]
#[command(name = "marea", version, about = "Scaffolding for marea apps")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate a new app workspace.
    New {
        /// Where to create it. The directory name is the project name unless
        /// --name says otherwise.
        path: PathBuf,

        /// Project name (kebab-case). Defaults to the directory name.
        #[arg(long)]
        name: Option<String>,

        /// Answer from a TOML file instead of prompting.
        #[arg(long, value_name = "FILE")]
        config: Option<PathBuf>,

        /// How generated manifests reference marea.
        #[arg(long, value_enum, default_value_t = DepModeArg::Auto)]
        deps: DepModeArg,

        /// Skip the post-generation build and architecture gates.
        #[arg(long)]
        no_verify: bool,

        /// Write into a directory that already has contents.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum DepModeArg {
    Auto,
    Path,
    Git,
}

impl From<DepModeArg> for DepMode {
    fn from(a: DepModeArg) -> Self {
        match a {
            DepModeArg::Auto => DepMode::Auto,
            DepModeArg::Path => DepMode::Path,
            DepModeArg::Git => DepMode::Git,
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Cmd::New {
            path,
            name,
            config,
            deps,
            no_verify,
            force,
        } => new(path, name, config, deps.into(), no_verify, force),
    }
}

fn new(
    path: PathBuf,
    name: Option<String>,
    config: Option<PathBuf>,
    deps: DepMode,
    no_verify: bool,
    force: bool,
) -> Result<()> {
    let target = absolute(&path)?;

    // `marea new Roommates` is the natural thing to type. The directory keeps
    // the name as given; the *project* name is derived from it, because it
    // feeds crate names and permanent identifiers that must be kebab-case.
    let raw_name = name.unwrap_or_else(|| {
        target
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    });
    let project_name = options::slugify(&raw_name);
    if project_name.is_empty() {
        bail!(
            "cannot derive a project name from {raw_name:?} — pass one with --name (lowercase letters, digits and single dashes, e.g. my-app)"
        );
    }

    let mut options = match &config {
        Some(file) => {
            let text = std::fs::read_to_string(file)
                .with_context(|| format!("reading {}", file.display()))?;
            let mut o: Options =
                toml::from_str(&text).with_context(|| format!("parsing {}", file.display()))?;
            if o.name.is_empty() {
                o.name = project_name.clone();
            } else {
                // Same normalisation the wizard applies, so a config file and
                // a typed answer behave identically.
                let slug = options::slugify(&o.name);
                if slug != o.name {
                    println!("  → project name: {slug} (from {:?})", o.name);
                    o.name = slug;
                }
            }
            o
        }
        None => prompts::ask(&project_name)?,
    };
    options.deps = deps;

    // The wizard validates each answer as it is given and never asks a
    // question whose answer is already implied, so this should only ever fire
    // for `--config`. It stays as the last line of defence for that path.
    if let Err(errs) = options.validate() {
        let mut msg = String::from("these answers do not go together:\n");
        for e in errs {
            msg.push_str(&format!("  · {e}\n"));
        }
        bail!(msg);
    }

    let marea = resolve_marea_source(&target, options.deps);
    match &marea {
        MareaSource::Path { prefix } => {
            println!("marea:  path deps against {prefix}");
        }
        MareaSource::Git { tag, .. } => {
            println!("marea:  git tag {tag} (no local checkout found)");
        }
    }

    let plan = render::plan(&options, marea)?;
    let written = write::write_plan(&plan, &target, force)?;
    println!("created {} files in {}", written.len(), target.display());

    if no_verify {
        println!("\nSkipped verification (--no-verify). Next: cargo check --workspace");
        return Ok(());
    }

    println!("\nVerifying…");
    let checks = verify::verify(&target, options.uses_wavesync())?;
    let mut failed = false;
    for c in &checks {
        if c.passed {
            println!("  ✓ {}", c.name);
        } else {
            failed = true;
            println!("  ✗ {}", c.name);
            for line in c.detail.lines().take(30) {
                println!("      {line}");
            }
        }
    }
    if failed {
        bail!("the generated workspace did not pass its own checks (see above)");
    }

    print_next_steps(&options, &target);
    Ok(())
}

fn resolve_marea_source(target: &Path, mode: DepMode) -> MareaSource {
    let git = || MareaSource::Git {
        url: MAREA_GIT.to_string(),
        tag: MAREA_TAG.to_string(),
    };
    match mode {
        DepMode::Git => git(),
        DepMode::Path | DepMode::Auto => match locate::find_marea(target) {
            Some(p) => MareaSource::Path {
                prefix: locate::relative_prefix(target, &p),
            },
            None => git(),
        },
    }
}

fn print_next_steps(options: &Options, target: &Path) {
    use marea_cli::manifest::app_crates;
    println!("\nNext:");
    println!("  cd {}", target.display());
    for app in app_crates(options) {
        let platform = match app.platform_feature() {
            "mobile" => "android",
            p => p,
        };
        println!(
            "  dx serve --package {} --platform {platform}",
            app.dir_name()
        );
    }
    println!("\nStart with crates/ui/assets/tokens.css — that file is the whole design system.");
}

fn absolute(p: &Path) -> Result<PathBuf> {
    Ok(if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()?.join(p)
    })
}
