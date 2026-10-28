//! lyk-steno-dict: generates a steno dictionary from phonetic rules.
//!
//! Step one: load the data and say what is in it, so every later rule can be judged against
//! Plover and Lapwing. Run `scripts/fetch-data.sh` first.

mod data;

use anyhow::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "data".into()));
    data::report(&root)
}
