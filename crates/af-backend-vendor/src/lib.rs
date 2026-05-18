// SPDX-License-Identifier: Apache-2.0
use af_backend::{AfBackend, BackendCapability, BackendReport};
use af_manifest::CoreManifest;
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct VendorBackend;

pub fn capabilities() -> Vec<BackendCapability> {
    vec![BackendCapability {
        name: "vendor-build".to_string(),
        supported: false,
        detail: Some(
            "Vendor backend crate is staged, but vendor synthesis/PnR orchestration is not implemented yet."
                .to_string(),
        ),
    }]
}

impl AfBackend for VendorBackend {
    fn name(&self) -> &'static str {
        "vendor"
    }

    fn doctor(&self) -> Result<BackendReport, af_backend::BackendError> {
        Ok(BackendReport::unavailable(
            self.name(),
            "vendor-toolchain",
            "Vendor backend crate is present, but vendor tool orchestration is not implemented yet.",
        ))
    }

    fn lint(
        &self,
        _manifest: &CoreManifest,
        _core_dir: &Path,
        _build_root: &Path,
    ) -> Result<BackendReport, af_backend::BackendError> {
        self.doctor()
    }

    fn sim(
        &self,
        _manifest: &CoreManifest,
        _core_dir: &Path,
        _build_root: &Path,
    ) -> Result<BackendReport, af_backend::BackendError> {
        self.doctor()
    }
}
