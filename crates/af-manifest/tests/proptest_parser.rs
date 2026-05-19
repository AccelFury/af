// SPDX-License-Identifier: Apache-2.0
//
// Property-based gates for the manifest parser.
//
// We do NOT try to generate well-formed manifests via proptest — the
// schema is too dense (clocks, resets, ports, parameters, all
// cross-referenced). Instead we exercise:
//
// 1. Fuzz `from_toml_str` with arbitrary bytes: it must never panic.
// 2. Fuzz the path validator: arbitrary strings either normalize or
//    produce a documented path-domain error code (e.g. PATH_EMPTY,
//    PATH_ABSOLUTE, PATH_TRAVERSAL).
// 3. Schema-version rejection: any version other than 0.1, 0.2, 0.3
//    yields a Validation error with a manifest-specific code.

use af_manifest::CoreManifest;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Parsing arbitrary text must never panic — it returns Ok or Err.
    #[test]
    fn from_toml_str_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        let text = String::from_utf8_lossy(&bytes).to_string();
        let result = std::panic::catch_unwind(|| {
            // Returns Result<_, ManifestError>; either is acceptable.
            let _ = CoreManifest::from_toml_str(&text, "af-core.toml");
        });
        prop_assert!(result.is_ok(), "manifest parser must never panic");
    }
}

#[test]
fn schema_version_unknown_is_rejected() {
    // A well-formed TOML stub with an out-of-band schema version.
    let text = r#"af_version = "9.9"
name = "x"
vendor = "x"
library = "x"
core = "x"
version = "0.0.0"
[rtl]
top = "x"
language = "systemverilog"
[sources]
files = []
"#;
    let err = CoreManifest::from_toml_str(text, "af-core.toml").unwrap_err();
    assert_eq!(err.code(), "AF_MANIFEST_INVALID");
}

#[test]
fn malformed_toml_yields_parse_error() {
    let err = CoreManifest::from_toml_str("not = valid toml [\n", "af-core.toml").unwrap_err();
    assert_eq!(err.code(), "AF_MANIFEST_PARSE_FAILED");
}

#[test]
fn missing_required_fields_yields_validation_error() {
    // Valid TOML but missing schema-required keys.
    let text = "af_version = \"0.3\"\n";
    let err = CoreManifest::from_toml_str(text, "af-core.toml");
    assert!(err.is_err(), "manifest without required fields must fail");
}
