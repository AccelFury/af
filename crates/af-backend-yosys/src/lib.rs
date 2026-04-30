// SPDX-License-Identifier: Apache-2.0
use af_backend::{AfBackend, BackendCapability, BackendReport};
use af_manifest::CoreManifest;
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct YosysBackend;

pub fn capabilities() -> Vec<BackendCapability> {
    vec![BackendCapability {
        name: "yosys-synthesis".to_string(),
        supported: false,
        detail: Some(
            "Yosys backend crate is staged, but synthesis orchestration is not implemented yet."
                .to_string(),
        ),
    }]
}

impl AfBackend for YosysBackend {
    fn name(&self) -> &'static str {
        "yosys"
    }

    fn doctor(&self) -> Result<BackendReport, af_backend::BackendError> {
        Ok(BackendReport::unavailable(
            self.name(),
            "yosys",
            "Yosys backend crate is present, but command orchestration is not implemented yet.",
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
