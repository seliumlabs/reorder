use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn cargo_bin() -> std::path::PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("reorder")
}

fn run_reorder(path: &Path) -> String {
    let bin_path = cargo_bin();
    let output = Command::new(&bin_path)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run reorder at {:?}: {}", bin_path, e));
    assert!(
        output.status.success(),
        "reorder failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read_to_string(path).expect("failed to read file")
}

fn test_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/regression");
    fs::create_dir_all(&dir).expect("failed to create test dir");
    dir
}

#[test]
fn test_type_aliases_no_extra_blank_lines() {
    let path = test_dir().join("types.rs");
    fs::write(
        &path,
        "\
use uuid::Uuid;

pub type RunId = Uuid;
pub type ArtifactId = Uuid;
pub type TransitionId = &'static str;
pub type ValidatorId = &'static str;
pub type ExecutorId = &'static str;
pub type FindingId = Uuid;
",
    )
    .expect("failed to write test file");

    let result = run_reorder(&path);

    assert_eq!(
        result,
        "\
use uuid::Uuid;

pub type ArtifactId = Uuid;
pub type ExecutorId = &'static str;
pub type FindingId = Uuid;
pub type RunId = Uuid;
pub type TransitionId = &'static str;
pub type ValidatorId = &'static str;
"
    );
}

#[test]
fn test_preserve_no_trailing_newline() {
    let path = test_dir().join("no_newline.rs");
    fs::write(
        &path,
        "\
use uuid::Uuid;

pub type RunId = Uuid;",
    )
    .expect("failed to write test file");

    let result = run_reorder(&path);

    assert!(
        !result.ends_with('\n'),
        "should not add trailing newline to file without one"
    );
}

#[test]
fn test_preserve_trailing_newline() {
    let path = test_dir().join("with_newline.rs");
    fs::write(
        &path,
        "\
use uuid::Uuid;

pub type RunId = Uuid;
",
    )
    .expect("failed to write test file");

    let result = run_reorder(&path);

    assert!(result.ends_with('\n'), "should preserve trailing newline");
    assert!(
        !result.ends_with("\n\n"),
        "should not add extra trailing newline"
    );
}

#[test]
fn test_no_extra_blank_line_after_last_item() {
    let path = test_dir().join("last_item.rs");
    fs::write(
        &path,
        "\
use uuid::Uuid;

pub type RunId = Uuid;

pub struct Foo {
    bar: i32,
}
",
    )
    .expect("failed to write test file");

    let result = run_reorder(&path);

    assert!(
        !result.ends_with("\n\n\n"),
        "should not have extra blank line after last item"
    );
}

#[test]
fn test_import_ordering() {
    let path = test_dir().join("imports.rs");
    fs::write(
        &path,
        "\
use uuid::Uuid;
use std::fs::File;
use crate::module::Blah;
use serde::Deserialize;
",
    )
    .expect("failed to write test file");

    let result = run_reorder(&path);

    assert_eq!(
        result,
        "\
use std::fs::File;

use serde::Deserialize;
use uuid::Uuid;

use crate::module::Blah;
"
    );
}

#[test]
fn test_modules_no_blank_lines_between() {
    let path = test_dir().join("modules.rs");
    fs::write(
        &path,
        "\
pub mod context;

pub mod ids;

pub mod journal;

pub mod run;
",
    )
    .expect("failed to write test file");

    let result = run_reorder(&path);

    assert_eq!(
        result,
        "\
pub mod context;
pub mod ids;
pub mod journal;
pub mod run;
"
    );
}
