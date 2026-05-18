// SPDX-License-Identifier: Apache-2.0
use af_complexity::{classify_path, ComplexityError, ProjectClass};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct SignoffPlanReport {
    pub generated_by: String,
    pub status: String,
    pub input: PathBuf,
    pub project_class: ProjectClass,
    pub board: Option<String>,
    pub checks: Vec<SignoffCheck>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct SignoffCheck {
    pub id: String,
    pub kind: String,
    pub required: bool,
    pub status: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Error)]
pub enum SignoffError {
    #[error(transparent)]
    Complexity(#[from] ComplexityError),
}

impl SignoffError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Complexity(err) => err.code(),
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::Complexity(err) => err.hint(),
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Complexity(err) => err.exit_code(),
        }
    }
}

pub fn create_signoff_plan(
    input: impl AsRef<Path>,
    project_class: Option<ProjectClass>,
    board: Option<String>,
) -> Result<SignoffPlanReport, SignoffError> {
    let input = input.as_ref();
    let classification = classify_path(input)?;
    let project_class = project_class.unwrap_or(classification.project_class);
    let mut warnings = classification.warnings;
    if board.is_some() && project_class < ProjectClass::SystemPlatform {
        warnings.push("Board-specific signoff requested for a core-level class; platform constraints remain planned.".to_string());
    }
    Ok(SignoffPlanReport {
        generated_by: "AccelFury IP Toolchain".to_string(),
        status: "planned".to_string(),
        input: input.to_path_buf(),
        project_class,
        board,
        checks: checks_for(project_class),
        warnings,
        limitations: vec![
            "Signoff plan generation is offline; it does not run lint, simulation, formal, timing, vendor tools, or hardware tests.".to_string(),
        ],
    })
}

fn checks_for(project_class: ProjectClass) -> Vec<SignoffCheck> {
    let ids = match project_class {
        ProjectClass::SimplePortable => vec!["manifest-check", "native-portable-lint", "smoke-sim"],
        ProjectClass::CompositePortable => vec![
            "manifest-check",
            "native-portable-lint",
            "smoke-sim",
            "dependency-check",
            "compatibility-check",
        ],
        ProjectClass::ComplexVendorAware => vec![
            "manifest-check",
            "native-portable-lint",
            "smoke-sim",
            "formal-targets",
            "backend-equivalence",
            "cdc-rdc-plan",
            "timing-plan",
            "resource-plan",
            "constructor-export",
        ],
        ProjectClass::SystemPlatform => vec![
            "platform-constraints",
            "hard-ip-integration",
            "board-integration",
            "security-production-flow",
            "timing-plan",
            "resource-plan",
        ],
        ProjectClass::ProductStack => vec![
            "catalog-export",
            "version-matrix",
            "compatibility-matrix",
            "release-reports",
            "known-limitations",
        ],
    };
    ids.into_iter()
        .map(|id| SignoffCheck {
            id: id.to_string(),
            kind: classify_check(id).to_string(),
            required: true,
            status: "planned".to_string(),
            evidence: Vec::new(),
        })
        .collect()
}

fn classify_check(id: &str) -> &'static str {
    if id.contains("security") {
        "security"
    } else if id.contains("timing") || id.contains("resource") || id.contains("platform") {
        "implementation"
    } else if id.contains("catalog") || id.contains("release") {
        "product"
    } else {
        "verification"
    }
}
