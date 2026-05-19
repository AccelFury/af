// SPDX-License-Identifier: Apache-2.0
#![no_main]

use af_ci::{generate_with_options, CiGenerateOptions};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let capped = text.chars().take(512).collect::<String>();
    let mut fields = capped.split('\0');
    let target = fields.next().unwrap_or_default();
    let backends = fields
        .next()
        .unwrap_or_default()
        .split([',', ' ', '\n', '\t'])
        .filter(|item| !item.is_empty())
        .map(|item| item.chars().take(64).collect::<String>())
        .collect::<Vec<_>>();
    let optional_fail_closed = data.first().is_some_and(|byte| byte & 1 == 1);
    let options = CiGenerateOptions {
        backends,
        optional_fail_closed,
    };
    let _ = generate_with_options(target, &options);
});
