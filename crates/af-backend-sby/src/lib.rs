// SPDX-License-Identifier: Apache-2.0
use af_backend::{AfBackend, BackendCapability, BackendReport};
use af_manifest::CoreManifest;
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct SbyBackend;

pub fn capabilities() -> Vec<BackendCapability> {
    vec![BackendCapability {
        name: "formal".to_string(),
        supported: false,
        detail: Some(
            "SymbiYosys backend crate is staged, but proof orchestration is not implemented yet."
                .to_string(),
        ),
    }]
}

impl AfBackend for SbyBackend {
    fn name(&self) -> &'static str {
        "sby"
    }

    fn doctor(&self) -> Result<BackendReport, af_backend::BackendError> {
        Ok(BackendReport::unavailable(
            self.name(),
            "sby",
            "SymbiYosys backend crate is present, but proof orchestration is not implemented yet.",
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
