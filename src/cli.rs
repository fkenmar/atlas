//! Pipeline driver. M0: hand-rolled flag parsing for the naive `map` path
//! (clap derive arrives in M1 with subcommands `map`/`serve`/`diff` —
//! deliberately *without* `--budget` until budgeting is real, which also
//! keeps the CI self-map smoke test gated).

use std::path::PathBuf;

use crate::lang::Language;

const USAGE: &str = "\
repomap — compile a codebase into a structural map (M0: naive full map)

USAGE:
    repomap [PATH] [--lang <csv>]

ARGS:
    PATH            repository root to map (default: .)

OPTIONS:
    --lang <csv>    restrict to extensions, e.g. --lang py,rs
    -h, --help      print this help
";

/// Parsed options for the default `map` subcommand.
pub struct MapOptions {
    pub root: PathBuf,
    /// Language restriction (`--lang py,ts`); empty = all Tier 1.
    pub langs: Vec<Language>,
}

/// Entry point called from `main`. Exits the process on usage errors.
pub fn run() {
    std::process::exit(match run_inner() {
        Ok(()) => 0,
        Err(code) => code,
    })
}

fn run_inner() -> Result<(), i32> {
    let options = parse_args(std::env::args().skip(1))?;

    let root = options.root.canonicalize().map_err(|err| {
        eprintln!("repomap: cannot open {}: {err}", options.root.display());
        2
    })?;
    let repo_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());

    let mut files = crate::discover::discover(&root);
    if !options.langs.is_empty() {
        files.retain(|f| options.langs.contains(&f.lang));
    }
    let outcome = crate::parse::parse_all(files);
    print!(
        "{}",
        crate::render::markdown::render_naive_map(&repo_name, &outcome)
    );
    Ok(())
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<MapOptions, i32> {
    let mut options = MapOptions {
        root: PathBuf::from("."),
        langs: Vec::new(),
    };
    let mut args = args.peekable();
    let mut saw_path = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Err(0);
            }
            "--lang" => {
                let Some(csv) = args.next() else {
                    eprintln!("repomap: --lang needs a value (e.g. --lang py,rs)");
                    return Err(2);
                };
                for token in csv.split(',') {
                    match Language::from_extension(token.trim()) {
                        Some(lang) => options.langs.push(lang),
                        None => {
                            eprintln!("repomap: unknown language extension {token:?}");
                            return Err(2);
                        }
                    }
                }
            }
            flag if flag.starts_with('-') => {
                eprintln!("repomap: unknown option {flag:?}\n\n{USAGE}");
                return Err(2);
            }
            path => {
                if saw_path {
                    eprintln!("repomap: more than one PATH given\n\n{USAGE}");
                    return Err(2);
                }
                saw_path = true;
                options.root = PathBuf::from(path);
            }
        }
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::parse_args;
    use crate::lang::Language;

    fn args(list: &[&str]) -> impl Iterator<Item = String> {
        list.iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn defaults_to_cwd() {
        let options = parse_args(args(&[])).unwrap();
        assert_eq!(options.root, std::path::PathBuf::from("."));
        assert!(options.langs.is_empty());
    }

    #[test]
    fn parses_path_and_lang_filter() {
        let options = parse_args(args(&["../somewhere", "--lang", "py,rs"])).unwrap();
        assert_eq!(options.root, std::path::PathBuf::from("../somewhere"));
        assert_eq!(options.langs, vec![Language::Python, Language::Rust]);
    }

    #[test]
    fn rejects_unknown_flag_and_bad_lang() {
        assert_eq!(parse_args(args(&["--budget", "1024"])).err(), Some(2));
        assert_eq!(parse_args(args(&["--lang", "cobol"])).err(), Some(2));
        assert_eq!(parse_args(args(&["a", "b"])).err(), Some(2));
    }
}
