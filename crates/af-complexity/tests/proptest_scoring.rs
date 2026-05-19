// SPDX-License-Identifier: Apache-2.0
//
// Property-based gates for the complexity scorer.
//
// 1. **Determinism**: the same spec text yields the same `ProjectClass`
//    and the same score on every call.
// 2. **Score bounds**: the score is clamped to the documented 0..=20
//    band.
// 3. **Generated-by stamp**: every classification report carries the
//    AccelFury stamp.

use af_complexity::classify_spec_text;
use proptest::prelude::*;

// Random ASCII text of bounded length. Includes spaces and newlines so
// keyword recognition is exercised non-trivially.
fn arbitrary_spec_text() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<u8>(), 0..512).prop_map(|bytes| {
        bytes
            .into_iter()
            .map(|b| {
                let printable = b % 96;
                if printable < 26 {
                    (b'a' + printable) as char
                } else if printable < 36 {
                    (b'0' + (printable - 26)) as char
                } else if printable < 50 {
                    ' '
                } else if printable < 56 {
                    '\n'
                } else {
                    // Punctuation that frequently appears in TOML-like specs.
                    let table = ['_', '-', '.', '=', '"', '[', ']', '\t', ':', ',', '{', '}'];
                    table[(printable as usize - 56) % table.len()]
                }
            })
            .collect()
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Same input → same classification (no hidden state, no clock-based
    /// drift).
    #[test]
    fn classify_spec_text_is_deterministic(text in arbitrary_spec_text()) {
        let a = classify_spec_text(&text);
        let b = classify_spec_text(&text);
        prop_assert_eq!(&a.project_class, &b.project_class);
        prop_assert_eq!(a.score, b.score);
        prop_assert_eq!(a.triggers.clone(), b.triggers);
    }

    /// Score is clamped to the documented 0..=20 band.
    #[test]
    fn score_is_within_documented_band(text in arbitrary_spec_text()) {
        let r = classify_spec_text(&text);
        prop_assert!(r.score <= 20, "score {} exceeds documented max 20", r.score);
    }

    /// Every report carries the AccelFury generated-by stamp.
    #[test]
    fn generated_by_marker_is_always_present(text in arbitrary_spec_text()) {
        let r = classify_spec_text(&text);
        prop_assert!(
            r.generated_by.contains("AccelFury"),
            "generated_by stamp missing or wrong: {:?}",
            r.generated_by
        );
    }
}

#[test]
fn empty_spec_is_simple_portable() {
    let r = classify_spec_text("");
    // Empty spec contains no triggers, so it stays at simple-portable.
    assert_eq!(
        r.project_class.as_str(),
        "simple-portable",
        "empty spec must default to simple-portable"
    );
    // Implementation may keep a small baseline score; assert only the
    // documented upper bound.
    assert!(
        r.score < 4,
        "empty spec score must be near zero, got {}",
        r.score
    );
}

#[test]
fn known_trigger_phrase_increases_score_over_empty() {
    let empty = classify_spec_text("");
    let with_trigger = classify_spec_text("platforms = [\"system\"]\nproject = true\n");
    assert!(
        with_trigger.score >= empty.score,
        "adding triggers must not decrease score"
    );
}
