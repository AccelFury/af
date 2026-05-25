// SPDX-License-Identifier: Apache-2.0
use crate::cores_registry;
use af_board_db::BoardEntry;
use af_manifest::CoreManifest;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

const TARGET: &str = "fpga.chat-v1";
const BOARD_REVISION_MISSING_REASON: &str = "revision_missing_from_upstream";
const NON_OSI_LICENSE_REASON: &str = "non_osi_license";
const OSI_LICENSES: &[&str] = &[
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "MIT",
    "MPL-2.0",
    "ISC",
    "0BSD",
];

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CatalogReadinessReport {
    pub target: String,
    pub status: String,
    pub board_records: BoardCatalogReadiness,
    pub core_licenses: CoreLicenseReadiness,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BoardCatalogReadiness {
    pub checked_count: usize,
    pub ready_count: usize,
    pub blocked_count: usize,
    pub blockers: Vec<BoardCatalogBlocker>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BoardCatalogBlocker {
    pub code: String,
    pub board_id: String,
    pub missing_fields: Vec<String>,
    pub reason: String,
    pub hint: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CoreLicenseReadiness {
    pub checked_count: usize,
    pub ready_count: usize,
    pub blocked_count: usize,
    pub blockers: Vec<CoreLicenseBlocker>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CoreLicenseBlocker {
    pub code: String,
    pub core_id: String,
    pub manifest_path: String,
    pub license: String,
    pub reason: String,
    pub hint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CoreLicenseRecord {
    core_id: String,
    manifest_path: String,
    license: String,
}

pub fn check(root: &Path) -> CatalogReadinessReport {
    let boards = af_board_db::load_registry_boards(root.join("registries/boards.registry.json"))
        .unwrap_or_default();
    let board_records = evaluate_board_records(&boards);
    let core_licenses = evaluate_core_licenses(&collect_core_license_records(root));
    let status = if board_records.blocked_count == 0 && core_licenses.blocked_count == 0 {
        "ready"
    } else {
        "blocked"
    };

    CatalogReadinessReport {
        target: TARGET.to_string(),
        status: status.to_string(),
        board_records,
        core_licenses,
    }
}

fn evaluate_board_records(boards: &[BoardEntry]) -> BoardCatalogReadiness {
    let mut blockers = Vec::new();

    for board in boards {
        let mut missing_fields = Vec::new();
        if is_blank(board.revision.as_deref()) {
            missing_fields.push("revision".to_string());
        }
        if is_blank(board.revision_source_locator.as_deref()) {
            missing_fields.push("revision_source_locator".to_string());
        }
        if !missing_fields.is_empty() {
            blockers.push(BoardCatalogBlocker {
                code: "AF_CATALOG_BOARD_REVISION_MISSING".to_string(),
                board_id: board.board_id.clone(),
                missing_fields,
                reason: BOARD_REVISION_MISSING_REASON.to_string(),
                hint: "Capture revision and revision_source_locator from an official schematic or product page before fpga.chat catalog export.".to_string(),
            });
        }
    }

    let blocked_count = blockers.len();
    BoardCatalogReadiness {
        checked_count: boards.len(),
        ready_count: boards.len().saturating_sub(blocked_count),
        blocked_count,
        blockers,
    }
}

fn evaluate_core_licenses(records: &[CoreLicenseRecord]) -> CoreLicenseReadiness {
    let mut blockers = Vec::new();

    for record in records {
        if !is_osi_license(&record.license) {
            blockers.push(CoreLicenseBlocker {
                code: "AF_CATALOG_CORE_LICENSE_NON_OSI".to_string(),
                core_id: record.core_id.clone(),
                manifest_path: record.manifest_path.clone(),
                license: record.license.clone(),
                reason: NON_OSI_LICENSE_REASON.to_string(),
                hint: "Publish shareable catalog cores under an OSI-approved license, or keep them deferred from fpga.chat.".to_string(),
            });
        }
    }

    let blocked_count = blockers.len();
    CoreLicenseReadiness {
        checked_count: records.len(),
        ready_count: records.len().saturating_sub(blocked_count),
        blocked_count,
        blockers,
    }
}

fn collect_core_license_records(root: &Path) -> Vec<CoreLicenseRecord> {
    let mut records = Vec::new();
    let mut seen_paths = BTreeSet::new();

    if let Ok(registry) = cores_registry::load(root) {
        for core in registry.cores {
            let Some(reference_path) = core.reference_path else {
                continue;
            };
            let manifest_path = root.join(&reference_path);
            if manifest_path.is_file() {
                push_manifest_record(
                    &mut records,
                    &mut seen_paths,
                    core.core_id,
                    reference_path,
                    &manifest_path,
                );
            }
        }
    }

    let examples_dir = root.join("examples");
    let Ok(entries) = fs::read_dir(examples_dir) else {
        return records;
    };
    for entry in entries.flatten() {
        let manifest_path = entry.path().join("af-core.toml");
        if !manifest_path.is_file() {
            continue;
        }
        let relative = manifest_path
            .strip_prefix(root)
            .map(portable_path)
            .unwrap_or_else(|_| portable_path(&manifest_path));
        let core_id = CoreManifest::from_path(&manifest_path)
            .map(|manifest| manifest.core)
            .unwrap_or_else(|_| entry.file_name().to_string_lossy().replace('-', "_"));
        push_manifest_record(
            &mut records,
            &mut seen_paths,
            core_id,
            relative,
            &manifest_path,
        );
    }

    records
}

fn push_manifest_record(
    records: &mut Vec<CoreLicenseRecord>,
    seen_paths: &mut BTreeSet<String>,
    core_id: String,
    manifest_path: String,
    path: &Path,
) {
    if !seen_paths.insert(manifest_path.clone()) {
        return;
    }
    if let Ok(manifest) = CoreManifest::from_path(path) {
        records.push(CoreLicenseRecord {
            core_id,
            manifest_path,
            license: manifest.metadata.license.unwrap_or_default(),
        });
    }
}

fn is_osi_license(license: &str) -> bool {
    let license = license.trim();
    OSI_LICENSES.contains(&license)
}

fn is_blank(value: Option<&str>) -> bool {
    value.map(str::trim).unwrap_or_default().is_empty()
}

fn portable_path(path: &Path) -> String {
    let parts: Vec<String> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            Component::CurDir => Some(".".to_string()),
            Component::ParentDir => Some("..".to_string()),
            Component::RootDir | Component::Prefix(_) => None,
        })
        .collect();
    if parts.is_empty() {
        PathBuf::from(path).display().to_string()
    } else {
        parts.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board_entry(revision: Option<&str>, revision_source_locator: Option<&str>) -> BoardEntry {
        BoardEntry {
            board_id: "demo_board".to_string(),
            display_name: "Demo Board".to_string(),
            vendor: "demo".to_string(),
            fpga_family: "demo_family".to_string(),
            fpga_part_if_known_or_template: "DEMO-1".to_string(),
            logic_size_class: "small".to_string(),
            dsp_class: "low".to_string(),
            memory_class: "none".to_string(),
            high_speed_io_class: "none".to_string(),
            default_toolchain: "demo_toolchain".to_string(),
            alternative_toolchains: Vec::new(),
            constraint_format: "xdc".to_string(),
            board_dir: "boards/demo/demo_board".to_string(),
            exact_pinout_status: "draft_placeholder".to_string(),
            revision: revision.map(str::to_string),
            revision_source_locator: revision_source_locator.map(str::to_string),
            safe_for_beginner: false,
            suggested_ip_classes: Vec::new(),
            excluded_ip_classes: Vec::new(),
            notes: String::new(),
        }
    }

    #[test]
    fn board_without_revision_fields_is_blocked() {
        let report = evaluate_board_records(&[board_entry(None, None)]);

        assert_eq!(report.checked_count, 1);
        assert_eq!(report.ready_count, 0);
        assert_eq!(report.blocked_count, 1);
        assert_eq!(report.blockers[0].code, "AF_CATALOG_BOARD_REVISION_MISSING");
        assert_eq!(
            report.blockers[0].missing_fields,
            vec!["revision", "revision_source_locator"]
        );
    }

    #[test]
    fn board_with_revision_fields_is_ready() {
        let report = evaluate_board_records(&[board_entry(
            Some("Rev 1.0"),
            Some("official schematic title block"),
        )]);

        assert_eq!(report.checked_count, 1);
        assert_eq!(report.ready_count, 1);
        assert_eq!(report.blocked_count, 0);
        assert!(report.blockers.is_empty());
    }

    #[test]
    fn source_available_license_is_blocked_for_catalog() {
        let report = evaluate_core_licenses(&[CoreLicenseRecord {
            core_id: "af_demo".to_string(),
            manifest_path: "examples/af-demo/af-core.toml".to_string(),
            license: "AccelFury Source Available License v1.0".to_string(),
        }]);

        assert_eq!(report.checked_count, 1);
        assert_eq!(report.ready_count, 0);
        assert_eq!(report.blocked_count, 1);
        assert_eq!(report.blockers[0].code, "AF_CATALOG_CORE_LICENSE_NON_OSI");
    }

    #[test]
    fn mit_license_is_ready_for_catalog() {
        let report = evaluate_core_licenses(&[CoreLicenseRecord {
            core_id: "af_demo".to_string(),
            manifest_path: "examples/af-demo/af-core.toml".to_string(),
            license: "MIT".to_string(),
        }]);

        assert_eq!(report.checked_count, 1);
        assert_eq!(report.ready_count, 1);
        assert_eq!(report.blocked_count, 0);
        assert!(report.blockers.is_empty());
    }
}
