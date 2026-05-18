// SPDX-License-Identifier: Apache-2.0

use serde_yaml::Value;
use std::collections::BTreeMap;

pub fn merge_workflow(
    existing: &str,
    generated: &str,
    allow_rewrite: bool,
) -> Result<(String, Vec<String>, Vec<String>), String> {
    let existing_value = parse_yaml(existing)?;
    let generated_value = parse_yaml(generated)?;

    let existing_jobs = existing_value
        .get("jobs")
        .and_then(Value::as_mapping)
        .cloned()
        .unwrap_or_default();
    let generated_jobs = generated_value
        .get("jobs")
        .and_then(Value::as_mapping)
        .cloned()
        .unwrap_or_default();

    let (merged_jobs, added, conflicts) =
        merge_job_maps(&existing_jobs, &generated_jobs, allow_rewrite);
    let mut merged = BTreeMap::new();
    for (key, value) in existing_value.as_mapping().cloned().unwrap_or_default() {
        let key = key.as_str().unwrap_or("").to_string();
        if key != "jobs" {
            merged.insert(key, value);
        }
    }
    merged.insert("jobs".to_string(), Value::Mapping(merged_jobs.clone()));

    let ordered = serde_yaml::to_string(&merged)
        .map_err(|err| format!("failed to serialize merged workflow: {err}"))?;

    Ok((ordered, added, conflicts))
}

fn merge_job_maps(
    existing_jobs: &serde_yaml::Mapping,
    generated_jobs: &serde_yaml::Mapping,
    allow_rewrite: bool,
) -> (serde_yaml::Mapping, Vec<String>, Vec<String>) {
    let mut merged = existing_jobs.clone();
    let mut added = Vec::new();
    let mut conflicts = Vec::new();

    for (name, job) in generated_jobs {
        let name = match name.as_str() {
            Some(name) => name.to_string(),
            None => continue,
        };
        let key = Value::String(name.clone());
        match merged.get(&key) {
            Some(_) if allow_rewrite => {
                merged.insert(key, job.clone());
                added.push(name);
            }
            Some(_) => conflicts.push(name),
            None => {
                merged.insert(key, job.clone());
                added.push(name);
            }
        }
    }

    (merged, added, conflicts)
}

fn parse_yaml(text: &str) -> Result<Value, String> {
    serde_yaml::from_str(text).map_err(|err| format!("invalid YAML: {err}"))
}
