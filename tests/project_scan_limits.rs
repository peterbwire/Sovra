//! Caller-selected discovery/read quotas for the partial project scanner.

use sovra::compiler::{
    project::{check_project, check_project_with_scan_limits},
    resources::ProjectScanLimits,
};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "svr-project-scan-quota-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("sovra.toml"),
            "[project]\nname = \"scan\"\nentry = \"main.svr\"\n",
        )
        .unwrap();
        Self(root)
    }
    fn write(&self, path: &str, source: impl AsRef<[u8]>) {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    fn root(&self) -> &Path {
        &self.0
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn unlimited() -> ProjectScanLimits {
    ProjectScanLimits {
        max_source_bytes: usize::MAX,
        max_total_source_bytes: usize::MAX,
        max_source_files: usize::MAX,
        max_directories: usize::MAX,
        max_directory_entries: usize::MAX,
    }
}

#[test]
fn exact_discovery_limits_preserve_project_and_each_dimension_rejects_overflow() {
    let fixture = Fixture::new();
    fixture.write("main.svr", "fn main() {}\n");
    fixture.write("sub/extra.svr", "fn extra() {}\n");
    fixture.write("sub/deep/leaf.svr", "fn leaf() {}\n");
    fixture.write("ignored.txt", "not source");
    // Root: manifest/main/sub/ignored; sub: extra/deep; deep: leaf.
    let exact = ProjectScanLimits {
        max_source_files: 3,
        max_directories: 3,
        max_directory_entries: 7,
        ..unlimited()
    };
    assert_eq!(
        check_project_with_scan_limits(fixture.root(), exact).unwrap(),
        check_project(fixture.root()).unwrap()
    );
    for limits in [
        ProjectScanLimits {
            max_source_files: 2,
            ..exact
        },
        ProjectScanLimits {
            max_directories: 2,
            ..exact
        },
        ProjectScanLimits {
            max_directory_entries: 6,
            ..exact
        },
    ] {
        let errors = check_project_with_scan_limits(fixture.root(), limits).unwrap_err();
        assert_eq!(errors.items.len(), 1);
        assert_eq!(errors.items[0].code, "E4150");
        assert!(errors.items[0].source_file.is_none());
        assert!(errors.items[0].message.contains("project scan"));
    }
    assert!(check_project(fixture.root()).is_ok());
}

#[test]
fn byte_limits_count_utf8_and_cumulative_reads_exactly() {
    let fixture = Fixture::new();
    let source = "// λ\nfn main() {}\n";
    fixture.write("main.svr", source);
    fixture.write("other.svr", "// λ\n");
    let total = source.len() + "// λ\n".len();
    let limits = ProjectScanLimits {
        max_source_bytes: source.len(),
        max_total_source_bytes: total,
        ..unlimited()
    };
    assert_eq!(
        check_project_with_scan_limits(fixture.root(), limits).unwrap(),
        check_project(fixture.root()).unwrap()
    );
    for (limits, kind) in [
        (
            ProjectScanLimits {
                max_source_bytes: source.len() - 1,
                ..limits
            },
            "source file byte",
        ),
        (
            ProjectScanLimits {
                max_total_source_bytes: total - 1,
                ..limits
            },
            "total source byte",
        ),
    ] {
        let errors = check_project_with_scan_limits(fixture.root(), limits).unwrap_err();
        assert_eq!(errors.items[0].code, "E4150");
        assert!(errors.items[0].message.contains(kind));
    }
}

#[test]
fn zero_and_empty_inputs_never_turn_exhaustion_into_success() {
    let fixture = Fixture::new();
    fixture.write("main.svr", "");
    let limits = ProjectScanLimits {
        max_source_bytes: 0,
        max_total_source_bytes: 0,
        max_source_files: 1,
        max_directories: 1,
        max_directory_entries: 2,
    };
    assert!(check_project_with_scan_limits(fixture.root(), limits).is_ok());
    for limits in [
        ProjectScanLimits {
            max_source_files: 0,
            ..limits
        },
        ProjectScanLimits {
            max_directories: 0,
            ..limits
        },
        ProjectScanLimits {
            max_directory_entries: 0,
            ..limits
        },
    ] {
        assert_eq!(
            check_project_with_scan_limits(fixture.root(), limits)
                .unwrap_err()
                .items[0]
                .code,
            "E4150"
        );
    }
    fixture.write("main.svr", "λ");
    assert_eq!(
        check_project_with_scan_limits(fixture.root(), limits)
            .unwrap_err()
            .items[0]
            .code,
        "E4150"
    );
}

#[test]
fn non_source_entries_count_and_byte_exhaustion_stops_before_later_bad_utf8() {
    let fixture = Fixture::new();
    fixture.write("main.svr", "fn main() {}\n");
    fixture.write("ignored.txt", "");
    let errors = check_project_with_scan_limits(
        fixture.root(),
        ProjectScanLimits {
            max_directory_entries: 2,
            ..unlimited()
        },
    )
    .unwrap_err();
    assert_eq!(errors.items[0].code, "E4150");
    fixture.write("z-invalid.svr", [0xff]);
    let errors = check_project_with_scan_limits(
        fixture.root(),
        ProjectScanLimits {
            max_total_source_bytes: 0,
            ..unlimited()
        },
    )
    .unwrap_err();
    assert_eq!(errors.items.len(), 1);
    assert_eq!(errors.items[0].code, "E4150");
    assert!(check_project(fixture.root())
        .unwrap_err()
        .items
        .iter()
        .any(|error| error.code == "E4009"));
}

#[test]
fn limited_scans_abort_on_failed_reads_without_reusing_the_byte_allowance() {
    let fixture = Fixture::new();
    fixture.write("main.svr", [0xff]);
    fixture.write("other.svr", [0xfe]);
    let errors = check_project_with_scan_limits(fixture.root(), unlimited()).unwrap_err();
    assert_eq!(errors.items.len(), 1);
    assert_eq!(errors.items[0].code, "E4009");
    let defaults = check_project(fixture.root()).unwrap_err();
    assert_eq!(
        defaults
            .items
            .iter()
            .filter(|error| error.code == "E4009")
            .count(),
        1
    );
}

#[test]
fn approved_byte_default_accepts_exact_limit_and_explicit_override() {
    let limits = ProjectScanLimits::DEFAULT;
    assert_eq!(limits.max_source_bytes, 16 * 1024 * 1024);
    assert_eq!(limits.max_total_source_bytes, 64 * 1024 * 1024);
    assert_eq!(limits.max_source_files, 4096);
    assert_eq!(limits.max_directories, 4096);
    assert_eq!(limits.max_directory_entries, 100_000);
    let fixture = Fixture::new();
    let mut source = vec![b' '; limits.max_source_bytes];
    fixture.write("main.svr", &source);
    assert!(check_project(fixture.root()).is_ok());
    source.push(b' ');
    fixture.write("main.svr", &source);
    assert_eq!(
        check_project(fixture.root()).unwrap_err().items[0].code,
        "E4150"
    );
    assert!(check_project_with_scan_limits(
        fixture.root(),
        ProjectScanLimits {
            max_source_bytes: limits.max_source_bytes + 1,
            ..limits
        }
    )
    .is_ok());
}

#[test]
fn approved_source_file_default_accepts_exact_count_and_rejects_overflow() {
    let fixture = Fixture::new();
    fixture.write("main.svr", "");
    for index in 1..ProjectScanLimits::DEFAULT.max_source_files {
        fixture.write(&format!("source{index:04}.svr"), "");
    }
    assert_eq!(
        check_project(fixture.root()).unwrap().source_files.len(),
        4096
    );
    fixture.write("overflow.svr", "");
    let errors = check_project(fixture.root()).unwrap_err();
    assert_eq!(errors.items[0].code, "E4150");
    assert!(errors.items[0].message.contains("source file count"));
}

#[test]
fn approved_directory_default_counts_root_and_empty_directories() {
    let fixture = Fixture::new();
    fixture.write("main.svr", "");
    for index in 1..ProjectScanLimits::DEFAULT.max_directories {
        std::fs::create_dir(fixture.0.join(format!("directory{index:04}"))).unwrap();
    }
    assert!(check_project(fixture.root()).is_ok());
    std::fs::create_dir(fixture.0.join("overflow")).unwrap();
    let errors = check_project(fixture.root()).unwrap_err();
    assert_eq!(errors.items[0].code, "E4150");
    assert!(errors.items[0].message.contains("directory count"));
}

#[cfg(unix)]
#[test]
fn source_symlink_aliases_consume_each_read_allowance() {
    let fixture = Fixture::new();
    let source = "fn main() {}\n";
    fixture.write("main.svr", source);
    std::os::unix::fs::symlink("main.svr", fixture.0.join("alias.svr")).unwrap();
    let limits = ProjectScanLimits {
        max_total_source_bytes: source.len() * 2,
        ..unlimited()
    };
    assert!(check_project_with_scan_limits(fixture.root(), limits).is_ok());
    assert_eq!(
        check_project_with_scan_limits(
            fixture.root(),
            ProjectScanLimits {
                max_total_source_bytes: source.len() * 2 - 1,
                ..limits
            }
        )
        .unwrap_err()
        .items[0]
            .code,
        "E4150"
    );
}
