// SPDX-License-Identifier: Apache-2.0

use crate::ci::config::CiConfig;

const ALLOWLIST: &[&str] = &[
    "artifacts/openfpga-ci/logs/tool-versions.txt",
    "artifacts/openfpga-ci/logs/*.log",
    "artifacts/openfpga-ci/logs/*.txt",
    "artifacts/openfpga-ci/synth/*.json",
    "artifacts/openfpga-ci/pnr/*.json",
    "artifacts/openfpga-ci/reports/*.json",
    "artifacts/openfpga-ci/SHA256SUMS",
];

pub fn allowlist() -> Vec<&'static str> {
    ALLOWLIST.to_vec()
}

pub fn is_allowed(path: &str) -> bool {
    if path == "." || path == "./" || path.contains("..") {
        return false;
    }

    for entry in ALLOWLIST {
        if entry.contains('*') {
            let mut parts = entry.splitn(2, '*');
            let prefix = parts.next().unwrap_or_default();
            let suffix = parts.next().unwrap_or_default();
            if path.starts_with(prefix) && path.ends_with(suffix) {
                return true;
            }
        }
    }

    ALLOWLIST.contains(&path)
}

pub fn artifact_paths(config: &CiConfig) -> Vec<String> {
    let root = config.sorted_artifacts_root();
    vec![
        format!("{root}/logs/tool-versions.txt"),
        format!("{root}/logs/sim.log"),
        format!("{root}/logs/synth.log"),
        format!("{root}/synth/*.json"),
        format!("{root}/pnr/*.json"),
        format!("{root}/reports/*.json"),
        format!("{root}/SHA256SUMS"),
    ]
}

pub fn artifact_paths_from_root(root: &str) -> Vec<String> {
    vec![
        format!("{root}/logs/tool-versions.txt"),
        format!("{root}/logs/sim.log"),
        format!("{root}/logs/synth.log"),
        format!("{root}/synth/*.json"),
        format!("{root}/pnr/*.json"),
        format!("{root}/reports/*.json"),
        format!("{root}/SHA256SUMS"),
    ]
}
