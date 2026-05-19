// SPDX-License-Identifier: Apache-2.0
//
// Vendor backend is detect-only by design (vendor RTL/synthesis is not
// invoked unless the user explicitly opts in). The test pins that the
// crate never tries to execute vendor binaries on its own.

use af_backend::AfBackend;
use af_backend_vendor::{capabilities, VendorBackend};

#[test]
fn name_is_vendor() {
    assert_eq!(VendorBackend.name(), "vendor");
}

#[test]
fn capabilities_advertise_no_supported_rows() {
    for cap in capabilities() {
        assert!(
            !cap.supported,
            "vendor backend must not advertise supported rows yet"
        );
    }
}

#[test]
fn doctor_returns_unavailable() {
    let report = VendorBackend.doctor().expect("doctor must not error");
    assert_eq!(report.status, af_backend::BackendStatus::Unavailable);
    assert!(
        report
            .tool_versions
            .iter()
            .any(|tv| tv.tool == "vendor-toolchain"),
        "vendor backend doctor must announce the vendor-toolchain placeholder"
    );
}
