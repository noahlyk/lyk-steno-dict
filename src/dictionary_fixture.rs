//! A full, production-scale dictionary build, shared by every test that needs the real, settled
//! dictionary rather than the rules in isolation. `generate()` at this scale takes a few seconds -
//! built once per test run via [`std::sync::LazyLock`] instead of once per test that needs it.

use crate::analysis::Setup;
use crate::generate::{self, Briefs, Generated};
use crate::stroke::Stroke;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

/// Matches `build`/`deploy`'s own default: a test against a smaller corpus can show a weaker
/// stand-in outline than the one the real, deployed dictionary actually gives a word (see
/// `docs/decisions.md`'s CLI corpus-size entry) - this fixture exists specifically so tests judge
/// the theory against what ships, not a shortcut.
const COMMON: usize = 100_000;

static BUILD: LazyLock<(Setup, Generated)> = LazyLock::new(|| {
    let setup = Setup::load(Path::new("data"), None, Some(COMMON)).expect("setup loads");
    let briefs = Briefs::built_in().expect("built-in briefs load");
    let generated = generate::generate(&setup, &briefs, COMMON).expect("generate succeeds");
    (setup, generated)
});

pub fn setup() -> &'static Setup {
    &BUILD.0
}

pub fn generated() -> &'static Generated {
    &BUILD.1
}

/// A fresh `word -> outline` map each call, borrowing from the one shared build - cheap (a hash
/// map over already-computed entries), so there is no need to share this part too.
pub fn cheapest() -> HashMap<&'static str, &'static [Stroke]> {
    generate::cheapest(&BUILD.1.entries)
}
