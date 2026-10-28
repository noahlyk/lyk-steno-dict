#!/bin/sh
# Download the data the generator reads into data/ (not committed: see .gitignore).
# Safe to run again: files that are already there are skipped.
set -eu

cd "$(dirname "$0")/.."
mkdir -p data/pronunciation data/frequency data/dictionaries

get() {
    out="$1"
    url="$2"
    if [ -s "$out" ]; then
        echo "have  $out"
        return
    fi
    echo "fetch $out"
    curl -fsSL --retry 3 -o "$out.part" "$url"
    mv "$out.part" "$out"
}

# Pronunciations. CMUdict is ARPAbet (stress digits on vowels); the MFA dictionaries are the
# largest English lists with a licence we can use (ARPAbet-like symbols in IPA, several
# pronunciations per word); WikiPron is crowd-sourced IPA from Wiktionary.
CMU=https://raw.githubusercontent.com/cmusphinx/cmudict/master
get data/pronunciation/cmudict.dict "$CMU/cmudict.dict"
get data/pronunciation/cmudict.phones "$CMU/cmudict.phones"
MFA=https://github.com/MontrealCorpusTools/mfa-models/releases/download
get data/pronunciation/english_us_mfa.dict "$MFA/dictionary-english_us_mfa-v3.1.0/english_us_mfa.dict"
get data/pronunciation/english_uk_mfa.dict "$MFA/dictionary-english_uk_mfa-v3.1.0/english_uk_mfa.dict"
WIKIPRON=https://raw.githubusercontent.com/CUNY-CL/wikipron/master/data/scrape/tsv
get data/pronunciation/wikipron_us_broad.tsv "$WIKIPRON/eng_latn_us_broad.tsv"
get data/pronunciation/wikipron_uk_broad.tsv "$WIKIPRON/eng_latn_uk_broad.tsv"

# How common each word is (OpenSubtitles 2018), to decide which words get the short outlines.
get data/frequency/en_full.txt \
    https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/2018/en/en_full.txt

# The dictionaries to match: Plover's own, and Lapwing's.
get data/dictionaries/plover-main.json \
    https://raw.githubusercontent.com/openstenoproject/plover/main/plover/assets/main.json
get data/dictionaries/plover-commands.json \
    https://raw.githubusercontent.com/openstenoproject/plover/main/plover/assets/commands.json
LAPWING=https://raw.githubusercontent.com/aerickt/plover-lapwing-aio/main/plover_lapwing/dictionaries
for name in lapwing-base lapwing-commands lapwing-numbers lapwing-uk-additions \
    lapwing-prefixed-proper-nouns lapwing-proper-nouns; do
    get "data/dictionaries/$name.json" "$LAPWING/$name.json"
done
