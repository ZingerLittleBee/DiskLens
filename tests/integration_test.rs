use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use disklens::models::node::{human_readable_size, Node};
use disklens::models::scan_result::ScanResult;
use disklens::models::index::{PathIndex, SizeIndex};
use disklens::core::analyzer::{Analyzer, MergedItem};
use disklens::config::settings::Settings;
use disklens::export::json::export_json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a unique temporary directory for a test.
fn make_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("disklens_test_{}", name));
    let _ = std::fs::remove_dir_all(&dir); // clean up from previous runs
    std::fs::create_dir_all(&dir).expect("create test dir");
    dir
}

/// Remove a temporary test directory.
fn cleanup(dir: &PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
}

/// Build a simple Node tree for testing (no filesystem needed).
fn sample_tree() -> Node {
    let file_a = Node::from_file(
        PathBuf::from("/test/a.txt"),
        "a.txt".into(),
        1000,
        Some(SystemTime::now()),
        Some(1),
    );
    let file_b = Node::from_file(
        PathBuf::from("/test/b.txt"),
        "b.txt".into(),
        2000,
        Some(SystemTime::now()),
        Some(2),
    );
    let file_c = Node::from_file(
        PathBuf::from("/test/sub/c.txt"),
        "c.txt".into(),
        500,
        Some(SystemTime::now()),
        Some(3),
    );
    let sub_dir = Node::from_directory(
        PathBuf::from("/test/sub"),
        "sub".into(),
        vec![file_c],
    );
    Node::from_directory(
        PathBuf::from("/test"),
        "test".into(),
        vec![file_a, file_b, sub_dir],
    )
}

/// Build a ScanResult wrapping a given root node.
fn make_scan_result(root: Node) -> ScanResult {
    ScanResult {
        total_size: root.size,
        total_files: root.file_count,
        total_dirs: root.dir_count,
        scan_duration: Duration::from_millis(42),
        errors: vec![],
        timestamp: SystemTime::now(),
        scan_path: root.path.clone(),
        root,
    }
}

// ---------------------------------------------------------------------------
// 1. test_scan_basic – scan a simple directory with the real Scanner
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_scan_basic() {
    let dir = make_test_dir("scan_basic");

    // Create files
    std::fs::write(dir.join("hello.txt"), "hello world").unwrap();
    std::fs::create_dir_all(dir.join("subdir")).unwrap();
    std::fs::write(dir.join("subdir/nested.txt"), "nested content").unwrap();

    let settings = Settings {
        max_depth: None,
        max_concurrent_io: 4,
        follow_symlinks: false,
        merge_threshold: 0.01,
        ignore_patterns: vec![],
        cache_dir: std::env::temp_dir().join("disklens_cache_test"),
        cache_max_size_mb: 64,
        cache_max_age_days: 1,
    };

    let (event_tx, _rx) = disklens::core::events::create_event_channel();
    let scanner = disklens::core::scanner::Scanner::new(settings, event_tx);
    let result = scanner.scan(dir.clone()).await.expect("scan should succeed");

    // Basic assertions
    assert!(result.total_size > 0, "total_size should be > 0");
    assert!(result.total_files >= 2, "should have at least 2 files");
    assert!(result.total_dirs >= 2, "should have at least 2 dirs (root + subdir)");
    assert_eq!(result.scan_path, dir);
    assert!(result.errors.is_empty(), "no errors expected");

    cleanup(&dir);
}

// ---------------------------------------------------------------------------
// 2. test_scan_empty_dir
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_scan_empty_dir() {
    let dir = make_test_dir("scan_empty");

    let settings = Settings {
        max_depth: None,
        max_concurrent_io: 4,
        follow_symlinks: false,
        merge_threshold: 0.01,
        ignore_patterns: vec![],
        cache_dir: std::env::temp_dir().join("disklens_cache_test"),
        cache_max_size_mb: 64,
        cache_max_age_days: 1,
    };

    let (event_tx, _rx) = disklens::core::events::create_event_channel();
    let scanner = disklens::core::scanner::Scanner::new(settings, event_tx);
    let result = scanner.scan(dir.clone()).await.expect("scan should succeed");

    assert_eq!(result.total_size, 0);
    assert_eq!(result.total_files, 0);
    assert!(result.total_dirs >= 1); // the root directory itself
    assert!(result.root.children.is_empty());

    cleanup(&dir);
}

// ---------------------------------------------------------------------------
// 3. test_node_percentage
// ---------------------------------------------------------------------------

#[test]
fn test_node_percentage() {
    let node = Node::from_file(
        PathBuf::from("/x"),
        "x".into(),
        250,
        None,
        None,
    );
    let pct = node.percentage(1000);
    assert!((pct - 25.0).abs() < f64::EPSILON, "250/1000 = 25%");

    // Edge case: total_size == 0
    assert_eq!(node.percentage(0), 0.0);
}

// ---------------------------------------------------------------------------
// 4. test_human_readable_size
// ---------------------------------------------------------------------------

#[test]
fn test_human_readable_size() {
    assert_eq!(human_readable_size(0), "0 B");
    assert_eq!(human_readable_size(512), "512 B");
    assert_eq!(human_readable_size(1023), "1023 B");
    assert_eq!(human_readable_size(1024), "1.00 KB");
    assert_eq!(human_readable_size(1536), "1.50 KB");
    assert_eq!(human_readable_size(1024 * 1024), "1.00 MB");
    assert_eq!(human_readable_size(1024 * 1024 * 1024), "1.00 GB");
    assert_eq!(human_readable_size(1024u64 * 1024 * 1024 * 1024), "1.00 TB");

    // Node method should agree
    let node = Node::from_file(PathBuf::from("/f"), "f".into(), 2048, None, None);
    assert_eq!(node.human_readable_size(), "2.00 KB");
}

// ---------------------------------------------------------------------------
// 5. test_sort_modes – sorting by size / name / modified
// ---------------------------------------------------------------------------

#[test]
fn test_sort_modes() {
    let mut root = sample_tree();

    // Sort by size descending (default)
    Analyzer::sort_by_size(&mut root);
    let names: Vec<&str> = root.children.iter().map(|c| c.name.as_str()).collect();
    // b.txt=2000, a.txt=1000, sub=500
    assert_eq!(names, vec!["b.txt", "a.txt", "sub"]);

    // Name-based sort (manual)
    root.children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    let names: Vec<&str> = root.children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["a.txt", "b.txt", "sub"]);
}

// ---------------------------------------------------------------------------
// 6. test_path_index – search paths
// ---------------------------------------------------------------------------

#[test]
fn test_path_index() {
    let root = sample_tree();
    let idx = PathIndex::build(&root);

    // Search for "c.txt"
    let results = idx.search("c.txt");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], PathBuf::from("/test/sub/c.txt"));

    // Search for "txt" should match all 3 files
    let results = idx.search("txt");
    assert_eq!(results.len(), 3);

    // Case-insensitive
    let results = idx.search("C.TXT");
    assert_eq!(results.len(), 1);

    // No match
    let results = idx.search("zzz");
    assert!(results.is_empty());
}

// ---------------------------------------------------------------------------
// 7. test_size_index – top_n
// ---------------------------------------------------------------------------

#[test]
fn test_size_index() {
    let root = sample_tree();
    let idx = SizeIndex::build(&root);

    // top 2 should be the two largest entries
    let top2 = idx.top_n(2);
    assert_eq!(top2.len(), 2);
    // Largest should be root (/test = 3500) followed by b.txt (2000)
    assert_eq!(top2[0].1, 3500);
    assert_eq!(top2[1].1, 2000);

    // top 0 should be empty
    assert!(idx.top_n(0).is_empty());

    // top 100 (more than entries) returns all
    let all = idx.top_n(100);
    assert_eq!(all.len(), 5); // root + a.txt + b.txt + sub + c.txt
}

// ---------------------------------------------------------------------------
// 8. test_export_json – JSON round-trip
// ---------------------------------------------------------------------------

#[test]
fn test_export_json() {
    let root = sample_tree();
    let result = make_scan_result(root);

    let dir = make_test_dir("export_json");
    let out_path = dir.join("report.json");

    export_json(&result, &out_path).expect("export should succeed");

    // Read back and deserialize
    let json_bytes = std::fs::read(&out_path).expect("read exported file");
    let restored: ScanResult = serde_json::from_slice(&json_bytes).expect("deserialize");

    assert_eq!(restored.total_size, result.total_size);
    assert_eq!(restored.total_files, result.total_files);
    assert_eq!(restored.total_dirs, result.total_dirs);
    assert_eq!(restored.root.name, "test");
    assert_eq!(restored.root.children.len(), 3);

    cleanup(&dir);
}

// ---------------------------------------------------------------------------
// 9. test_analyzer_merge – merge_small_items
// ---------------------------------------------------------------------------

#[test]
fn test_analyzer_merge() {
    let root = sample_tree(); // total 3500 bytes

    // threshold 0.5 means items must be >= 50% to stay individual
    // Only root items: a.txt=1000 (28.6%), b.txt=2000 (57.1%), sub=500 (14.3%)
    let items = Analyzer::merge_small_items(&root, 0.5);

    // b.txt >= 50%, so it stays. a.txt and sub get merged.
    let individual: Vec<&MergedItem> = items.iter().filter(|i| !i.is_merged).collect();
    let merged: Vec<&MergedItem> = items.iter().filter(|i| i.is_merged).collect();

    assert_eq!(individual.len(), 1);
    assert_eq!(individual[0].name, "b.txt");
    assert!((individual[0].percentage - 57.142857142857146).abs() < 0.01);

    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].name, "Others");
    assert_eq!(merged[0].merged_count, 2);
    assert_eq!(merged[0].size, 1500); // a.txt + sub

    // Empty node
    let empty = Node::from_directory(PathBuf::from("/e"), "e".into(), vec![]);
    assert!(Analyzer::merge_small_items(&empty, 0.01).is_empty());
}

// ---------------------------------------------------------------------------
// 10. test_settings_default
// ---------------------------------------------------------------------------

#[test]
fn test_settings_default() {
    let s = Settings::default();

    assert!(s.max_depth.is_none());
    assert!(!s.follow_symlinks);
    assert!((s.merge_threshold - 0.01).abs() < f64::EPSILON);
    assert!(s.ignore_patterns.is_empty());
    assert!(s.max_concurrent_io > 0);
    assert_eq!(s.cache_max_size_mb, 512);
    assert_eq!(s.cache_max_age_days, 7);
}
