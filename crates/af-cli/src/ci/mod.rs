// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code, unused_imports)]

pub mod artifacts;
pub mod config;
pub mod detector;
pub mod diagnostics;
pub mod policy;
pub mod profiles;
pub mod renderer;
pub mod report;
pub mod scanner;
pub mod updater;
pub mod workflow;

pub use artifacts::*;
pub use config::*;
pub use detector::*;
pub use policy::*;
pub use profiles::*;
pub use renderer::*;
pub use report::*;
pub use scanner::*;
pub use updater::*;
pub use workflow::*;

pub fn safe_file_name(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

pub fn normalize_command(command: &str) -> String {
    command.trim().replace('\n', " ")
}
