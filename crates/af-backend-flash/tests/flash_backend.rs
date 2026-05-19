// SPDX-License-Identifier: Apache-2.0
//
// `af-backend-flash` is a staged backend: every entry-point reports
// `unavailable` rather than panicking. The test pins that contract so
// upstream callers can safely route to it without prior probing.

use af_backend::AfBackend;
use af_backend_flash::{capabilities, FlashBackend};

#[test]
fn name_is_flash() {
    assert_eq!(FlashBackend.name(), "flash");
}

#[test]
fn capabilities_are_declared_unsupported() {
    let caps = capabilities();
    assert!(!caps.is_empty(), "must declare at least one capability row");
    for cap in &caps {
        assert!(
            !cap.supported,
            "flash backend rows are still planned: {cap:?}"
        );
        assert!(cap.detail.is_some(), "must explain why not supported");
    }
}

#[test]
fn doctor_returns_unavailable_status_not_panic() {
    let report = FlashBackend.doctor().expect("doctor must not error");
    assert_eq!(report.status, af_backend::BackendStatus::Unavailable);
}
