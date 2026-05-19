// SPDX-License-Identifier: Apache-2.0
//
// Lock the BRAM accounting formula. Implementation:
//
//     bram_blocks = ceil((width * depth) / 18_432).max(1)
//
// summed over every `[[resources.memory]]` entry. 18_432 bits is the
// effective tile size used by the offline estimator. Any change to the
// constant or to the rounding strategy is a public-contract change for
// every downstream consumer that reads `resources["bram"].estimated`.

mod common;

use af_resource_model::plan_resources;
use common::{append_to_manifest, clone_example};

fn memory_fragment(name: &str, width: u32, depth: u32, kind: &str, policy: &str) -> String {
    format!(
        "\n[[resources.memory]]\nname = \"{name}\"\nkind = \"{kind}\"\nwidth = {width}\ndepth = {depth}\nbackend_policy = \"{policy}\"\n"
    )
}

fn bram_estimate(tmp_path: &std::path::Path) -> u64 {
    let report = plan_resources(tmp_path, None, None, None).expect("plan_resources runs");
    report
        .resources
        .get("bram")
        .map(|r| r.estimated)
        .unwrap_or_else(|| {
            panic!(
                "bram resource missing from estimate: {:?}",
                report.resources
            )
        })
}

#[test]
fn single_tile_at_threshold() {
    // 18_432 bits = exactly one tile.
    let tmp = clone_example("af-mod-add");
    append_to_manifest(
        tmp.path(),
        &memory_fragment("m0", 1, 18_432, "ram_sp", "portable"),
    );
    assert_eq!(
        bram_estimate(tmp.path()),
        1,
        "exactly-tile must yield 1 block"
    );
}

#[test]
fn just_over_threshold_rounds_up() {
    // 18_433 bits = 2 tiles (div_ceil).
    let tmp = clone_example("af-mod-add");
    append_to_manifest(
        tmp.path(),
        &memory_fragment("m0", 1, 18_433, "ram_sp", "portable"),
    );
    assert_eq!(bram_estimate(tmp.path()), 2, "div_ceil must round up");
}

#[test]
fn small_memory_clamps_to_minimum_of_one() {
    // 8 * 16 = 128 bits — far below the tile, but the .max(1) floor
    // applies so a declared memory is never reported as 0 blocks.
    let tmp = clone_example("af-mod-add");
    append_to_manifest(
        tmp.path(),
        &memory_fragment("m0", 8, 16, "ram_sp", "portable"),
    );
    assert_eq!(bram_estimate(tmp.path()), 1, "min-one floor must hold");
}

#[test]
fn memory_estimates_are_summed() {
    let tmp = clone_example("af-mod-add");
    // Three independent memories: 1 + 2 + 1 = 4 tiles.
    append_to_manifest(
        tmp.path(),
        &memory_fragment("m0", 1, 18_432, "ram_sp", "portable"),
    );
    append_to_manifest(
        tmp.path(),
        &memory_fragment("m1", 1, 18_433, "ram_sp", "portable"),
    );
    append_to_manifest(
        tmp.path(),
        &memory_fragment("m2", 8, 16, "ram_sp", "portable"),
    );
    assert_eq!(
        bram_estimate(tmp.path()),
        4,
        "BRAM blocks must sum across entries"
    );
}

#[test]
fn lut_estimate_is_always_present() {
    // Regardless of resource contracts, the offline model emits a `lut`
    // row with `policy = "portable"` and `uncertainty = "high"`.
    let tmp = clone_example("af-mod-add");
    let report = plan_resources(tmp.path(), None, None, None).expect("plan_resources runs");
    let lut = report.resources.get("lut").expect("lut row present");
    assert_eq!(lut.policy, "portable");
    assert_eq!(lut.uncertainty, "high");
    assert!(lut.estimated >= 16, "lut floor is 16: {}", lut.estimated);
}
