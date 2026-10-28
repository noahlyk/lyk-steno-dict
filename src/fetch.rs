//! `fetch`: download the data the generator reads into `data/`.
//!
//! American English only. Files that are already there are skipped, so it is safe to run again.

use anyhow::{Context, Result};
use std::fs;
use std::io;
use std::path::Path;

const CMU: &str = "https://raw.githubusercontent.com/cmusphinx/cmudict/master";
const MFA: &str = "https://github.com/MontrealCorpusTools/mfa-models/releases/download";
const WIKIPRON: &str = "https://raw.githubusercontent.com/CUNY-CL/wikipron/master/data/scrape/tsv";
const PLOVER: &str = "https://raw.githubusercontent.com/openstenoproject/plover/main/plover/assets";
const LAPWING: &str =
    "https://raw.githubusercontent.com/aerickt/plover-lapwing-aio/main/plover_lapwing/dictionaries";

/// Where each file goes under the data folder, and where it comes from.
fn sources() -> Vec<(String, String)> {
    let mut files = vec![
        // Pronunciations. CMUdict is ARPAbet with stress digits on the vowels. The MFA
        // dictionary is IPA with several pronunciations per word. WikiPron is IPA from Wiktionary.
        ("pronunciation/cmudict.dict".into(), format!("{CMU}/cmudict.dict")),
        ("pronunciation/cmudict.phones".into(), format!("{CMU}/cmudict.phones")),
        (
            "pronunciation/english_us_mfa.dict".into(),
            format!("{MFA}/dictionary-english_us_mfa-v3.1.0/english_us_mfa.dict"),
        ),
        (
            "pronunciation/wikipron_us_broad.tsv".into(),
            format!("{WIKIPRON}/eng_latn_us_broad.tsv"),
        ),
        // How common each word is (OpenSubtitles 2018), to decide who gets the short outlines
        (
            "frequency/en_full.txt".into(),
            "https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/2018/en/en_full.txt"
                .into(),
        ),
        // The dictionaries to match
        ("dictionaries/plover-main.json".into(), format!("{PLOVER}/main.json")),
        ("dictionaries/plover-commands.json".into(), format!("{PLOVER}/commands.json")),
    ];
    for name in [
        "lapwing-base",
        "lapwing-commands",
        "lapwing-numbers",
        "lapwing-prefixed-proper-nouns",
        "lapwing-proper-nouns",
    ] {
        files.push((
            format!("dictionaries/{name}.json"),
            format!("{LAPWING}/{name}.json"),
        ));
    }
    files
}

pub fn run(root: &Path) -> Result<()> {
    for (path, url) in sources() {
        let target = root.join(&path);
        if fs::metadata(&target).is_ok_and(|m| m.len() > 0) {
            println!("have  {path}");
            continue;
        }
        println!("fetch {path}");
        download(&url, &target).with_context(|| format!("{url} -> {path}"))?;
    }
    Ok(())
}

/// Download to a temporary name and rename, so a failed download never leaves half a file.
fn download(url: &str, target: &Path) -> Result<()> {
    if let Some(dir) = target.parent() {
        fs::create_dir_all(dir)?;
    }
    let part = target.with_extension("part");
    let response = ureq::get(url).call()?;
    let mut file = fs::File::create(&part)?;
    io::copy(&mut response.into_reader(), &mut file)?;
    fs::rename(&part, target)?;
    Ok(())
}
