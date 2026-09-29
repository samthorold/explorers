//! Export a config the genesis search or the research sweeps name as a
//! [`WorldRecipe`] the app opens unchanged (#581).
//!
//! The app loads only a `WorldRecipe` (`--recipe`). An atlas's live cells are
//! unit vectors only this crate can decode, and the research configs
//! (`sample:31`, `sample@9421:12`, `atlas:2`, …) exist only inside the
//! research bins. This bin decodes one of them here, in the search crate, and
//! writes the recipe the app reads — the app gains no search dependency.
//!
//! ## References
//!
//! - `cell:I,J,K` — the atlas live cell at behaviour-axis coordinate
//!   `[I, J, K]` (the `cell` field of an `atlas.json` cell).
//! - `atlas:N` — the `N`-th entry of `atlas.json`'s `cells` list: an atlas
//!   live cell by index, the same config `atlas:N` names in the research bins.
//! - `sample:i` — the `i`-th config of the shared seed-421 LHS draw.
//! - `sample@S:i` — the `i`-th config of the seed-`S` LHS draw.
//!
//! `atlas:N`, `sample:i` and `sample@S:i` are the selector keys
//! `permanence_crosscheck --configs` accepts, parsed by the same
//! [`parse_config_key`](explorers_search::config_source::parse_config_key)
//! and decoded the same way, so one reference names one world in both bins.
//! Atlas cells decode over the atlas's own search box; sample draws over the
//! full box. The atlas file is read only for `cell:` and `atlas:` references.
//! Its dead frontier is stored as counts only, so frontier configs cannot be
//! exported from it — the named research configs cover those cases.
//!
//! ## Run
//!
//! ```text
//! cargo run --release -p explorers-search --bin export_recipe -- \
//!     sample@9421:12 --out target/sample-9421-12.json
//! cargo run --release -p explorers-search --bin export_recipe -- \
//!     cell:3,1,0 --atlas atlas.json --out target/cell.json
//! cargo run --release -p explorers-app -- --recipe target/sample-9421-12.json
//! ```
//!
//! Flags: `--atlas PATH` (default `atlas.json`), `--out PATH` (default
//! `recipe.json`), `--max-ticks N` (default: the horizon the atlas records for
//! its cells, else the search's `T = 2000`). A reference that names no config
//! — a cell the atlas did not fill, an index out of range — fails with a
//! message naming it and a non-zero exit.

use std::path::PathBuf;

use explorers_search::atlas_file::read_atlas;
use explorers_search::recipe_export::{RecipeRef, export_recipe, write_recipe};

const USAGE: &str = "usage: export_recipe <cell:I,J,K|atlas:N|sample:i|sample@S:i> [--atlas PATH] [--out PATH] [--max-ticks N]";

/// Parse `argv`, export the recipe it names and write it; returns the path
/// written.
fn run<I: IntoIterator<Item = String>>(argv: I) -> Result<PathBuf, String> {
    let mut reference: Option<RecipeRef> = None;
    let mut atlas_path = PathBuf::from("atlas.json");
    let mut out = PathBuf::from("recipe.json");
    let mut max_ticks = None;
    let mut it = argv.into_iter();
    while let Some(arg) = it.next() {
        let mut value = |flag: &str| it.next().ok_or_else(|| format!("{flag} needs a value"));
        match arg.as_str() {
            "--atlas" => atlas_path = PathBuf::from(value("--atlas")?),
            "--out" => out = PathBuf::from(value("--out")?),
            "--max-ticks" => {
                let raw = value("--max-ticks")?;
                let ticks: u64 = raw
                    .parse()
                    .map_err(|_| format!("--max-ticks {raw:?} is not an integer"))?;
                max_ticks = Some(ticks);
            }
            flag if flag.starts_with("--") => {
                return Err(format!("unknown argument {flag:?}\n{USAGE}"));
            }
            raw if reference.is_none() => reference = Some(raw.parse()?),
            extra => return Err(format!("unexpected argument {extra:?}\n{USAGE}")),
        }
    }
    let reference = reference.ok_or(USAGE)?;
    let atlas = if reference.names_atlas_cell() {
        Some(read_atlas(&atlas_path).map_err(|e| format!("{reference}: {e}"))?)
    } else {
        None
    };
    let recipe = export_recipe(&reference, atlas.as_ref(), max_ticks)?;
    write_recipe(&recipe, &out)?;
    Ok(out)
}

fn main() {
    match run(std::env::args().skip(1)) {
        Ok(path) => eprintln!("export_recipe: wrote {}", path.display()),
        Err(e) => {
            eprintln!("export_recipe: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorers_search::recipe_export::export_recipe;
    use explorers_sim::WorldRecipe;

    fn argv(raw: &str) -> Vec<String> {
        raw.split_whitespace().map(String::from).collect()
    }

    /// The bin writes the recipe the reference names, at the horizon given,
    /// to `--out`; a sample reference needs no atlas file.
    #[test]
    fn writes_the_recipe_a_reference_names() {
        let out =
            std::env::temp_dir().join(format!("export-recipe-bin-{}.json", std::process::id()));
        let written = run(argv(&format!(
            "sample@9421:12 --out {} --max-ticks 600 --atlas no-such-atlas.json",
            out.display()
        )))
        .unwrap();
        assert_eq!(written, out);
        let back: WorldRecipe =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        let _ = std::fs::remove_file(&out);
        let expect = export_recipe(&"sample@9421:12".parse().unwrap(), None, Some(600)).unwrap();
        assert_eq!(back, expect);
    }

    /// Bad input is an error message naming what was wrong — never a panic.
    #[test]
    fn bad_input_fails_with_a_message() {
        let err = |raw: &str| run(argv(raw)).unwrap_err();
        assert!(err("").contains("usage"), "{}", err(""));
        assert!(err("sample:x").contains("sample:x") || err("sample:x").contains("\"x\""));
        assert!(err("sample:1 --bogus").contains("--bogus"));
        assert!(err("sample:1 --max-ticks").contains("--max-ticks"));
        assert!(err("sample:1 --max-ticks nope").contains("nope"));
        let missing = err("atlas:0 --atlas no-such-atlas.json");
        assert!(missing.contains("no-such-atlas.json"), "{missing}");
    }
}
