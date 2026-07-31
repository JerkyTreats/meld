//! Loose structural parity assertions over the resurrected README baseline corpus.
//!
//! The corpus at `tests/fixtures/readme_parity/` holds frozen input filesystems and
//! golden expected READMEs recovered from the deleted eval harness. Parity is loose
//! and structural: heading coverage against the golden README plus required mentions
//! from each fixture's researched expected properties. Byte equality is never asserted
//! because provider output is nondeterministic.

use std::fs;
use std::path::PathBuf;

pub(crate) const MIN_HEADING_COVERAGE: f64 = 0.70;

pub(crate) struct FixtureExpectation {
    pub case_id: &'static str,
    /// Substrings the generated root README must contain, from fixture_meta expected_properties.
    pub required_mentions: &'static [&'static str],
}

pub(crate) const FIXTURE_EXPECTATIONS: &[FixtureExpectation] = &[
    FixtureExpectation {
        case_id: "sample_nested",
        required_mentions: &["pkg"],
    },
    FixtureExpectation {
        case_id: "ghdocs_events",
        required_mentions: &["events", "components"],
    },
    FixtureExpectation {
        case_id: "ghdocs_deployments",
        required_mentions: &["production", "staging"],
    },
];

pub(crate) fn fixture_root(case_id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/readme_parity")
        .join(case_id)
}

pub(crate) fn expected_readme(case_id: &str) -> String {
    let path = fixture_root(case_id).join("expected/README.md");
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("golden README missing at {}: {error}", path.display()))
}

/// Markdown ATX headings, normalized to lowercase text without the marker.
pub(crate) fn headings(markdown: &str) -> Vec<String> {
    markdown
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let stripped = trimmed.trim_start_matches('#');
            if stripped.len() < trimmed.len() && !stripped.trim().is_empty() {
                Some(stripped.trim().to_lowercase())
            } else {
                None
            }
        })
        .collect()
}

/// Fraction of golden headings present in the generated document, matched loosely:
/// a golden heading counts as covered when any generated heading contains it or it
/// contains any generated heading.
pub(crate) fn heading_coverage(golden: &str, generated: &str) -> f64 {
    let golden_headings = headings(golden);
    if golden_headings.is_empty() {
        return 1.0;
    }
    let generated_headings = headings(generated);
    let covered = golden_headings
        .iter()
        .filter(|golden_heading| {
            generated_headings.iter().any(|generated_heading| {
                generated_heading.contains(golden_heading.as_str())
                    || golden_heading.contains(generated_heading.as_str())
            })
        })
        .count();
    covered as f64 / golden_headings.len() as f64
}

pub(crate) struct ParityFailure {
    pub reasons: Vec<String>,
}

/// Loose parity verdict for a generated root README against a fixture's golden README.
pub(crate) fn check_loose_parity(case_id: &str, generated: &str) -> Result<(), ParityFailure> {
    let mut reasons = Vec::new();
    let generated_lower = generated.to_lowercase();

    if generated.trim().is_empty() {
        reasons.push("generated README is empty".to_string());
    }

    let coverage = heading_coverage(&expected_readme(case_id), generated);
    if coverage < MIN_HEADING_COVERAGE {
        reasons.push(format!(
            "heading coverage {coverage:.2} below minimum {MIN_HEADING_COVERAGE:.2}"
        ));
    }

    let expectation = FIXTURE_EXPECTATIONS
        .iter()
        .find(|expectation| expectation.case_id == case_id)
        .unwrap_or_else(|| panic!("no expectation registered for fixture '{case_id}'"));
    for mention in expectation.required_mentions {
        if !generated_lower.contains(&mention.to_lowercase()) {
            reasons.push(format!("required mention '{mention}' absent"));
        }
    }

    if reasons.is_empty() {
        Ok(())
    } else {
        Err(ParityFailure { reasons })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The only surviving specimen of the hollow-README failure: well-formed prose
    /// narrating its own vacuity. Parity must reject it.
    const HOLLOW_SPECIMEN: &str = "# Verification Report\n\n## Scope\n\nNo claims provided in evidence_map claims array.\n\n## Caveats\n\nNo verified claims available; report contains no claims to verify.\n";

    #[test]
    fn golden_readmes_pass_their_own_parity_check() {
        for expectation in FIXTURE_EXPECTATIONS {
            let golden = expected_readme(expectation.case_id);
            if let Err(failure) = check_loose_parity(expectation.case_id, &golden) {
                panic!(
                    "golden README for '{}' failed its own parity check: {:?}",
                    expectation.case_id, failure.reasons
                );
            }
        }
    }

    #[test]
    fn hollow_specimen_fails_parity_for_every_fixture() {
        for expectation in FIXTURE_EXPECTATIONS {
            assert!(
                check_loose_parity(expectation.case_id, HOLLOW_SPECIMEN).is_err(),
                "hollow specimen passed parity for '{}'",
                expectation.case_id
            );
        }
    }

    #[test]
    fn empty_output_fails_parity() {
        assert!(check_loose_parity("sample_nested", "  \n").is_err());
    }

    #[test]
    fn heading_coverage_is_total_on_identical_documents() {
        let golden = expected_readme("ghdocs_events");
        assert!((heading_coverage(&golden, &golden) - 1.0).abs() < f64::EPSILON);
    }
}
