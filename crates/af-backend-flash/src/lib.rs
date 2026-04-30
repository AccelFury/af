// SPDX-License-Identifier: Apache-2.0
use af_backend::{AfBackend, BackendCapability, BackendReport};
use af_manifest::CoreManifest;
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct FlashBackend;

pub fn capabilities() -> Vec<BackendCapability> {
    vec![BackendCapability {
        name: "programmer-flash".to_string(),
        supported: false,
        detail: Some(
            "Flash backend crate is staged, but programmer orchestration is not implemented yet."
                .to_string(),
        ),
    }]
}

impl AfBackend for FlashBackend {
    fn name(&self) -> &'static str {
        "flash"
    }

    fn doctor(&self) -> Result<BackendReport, af_backend::BackendError> {
        Ok(BackendReport::unavailable(
            self.name(),
            "programmer",
            "Flash backend crate is present, but programmer orchestration is not implemented yet.",
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
