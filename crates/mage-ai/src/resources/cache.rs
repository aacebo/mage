use std::path::PathBuf;

/// Where downloaded model weights live: `$MAGE_CACHE`, else a `.cache/` directory at the root of
/// the enclosing repository, else `hf-hub`'s own default (`$HF_HOME`, else `~/.cache/huggingface`).
///
/// The fallback matters: a published `mage-ai` may run with no repository above it at all.
pub fn dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("MAGE_CACHE") {
        return Some(PathBuf::from(path));
    }

    root().map(|root| root.join(".cache"))
}

/// `.git` marks the repository root. Falling back to the *outermost* `Cargo.toml` finds the
/// workspace root -- the nearest one would resolve to whichever crate happens to be the cwd.
fn root() -> Option<PathBuf> {
    let current = std::env::current_dir().ok()?;

    if let Some(git) = current.ancestors().find(|dir| dir.join(".git").exists()) {
        return Some(git.to_path_buf());
    }

    current
        .ancestors()
        .filter(|dir| dir.join("Cargo.toml").exists())
        .last()
        .map(PathBuf::from)
}
