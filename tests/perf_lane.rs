//! Extraction benchmarks against a generated corpus.
//!
//! Verifies [NFR-003](../spec/non-functional/NFR-003-extraction-time-budget.md)
//! — full-batch time, single-file re-extraction latency, peak memory and
//! fixpoint convergence — and reports the corpus-scale recall figure for
//! [NFR-004](../spec/non-functional/NFR-004-conservative-resolution-precision.md).
//!
//! The corpus is *generated* rather than committed. A 500,000-line fixture
//! checked into git would dominate the repository, would still not resemble any
//! real codebase, and could not be regenerated at a different size when the
//! budget changes. Generation is deterministic — the shapes and counts are a
//! pure function of the requested size, with no randomness — so successive runs
//! measure the same work, which is what makes the numbers comparable at all.
//!
//! Run with `make bench`, which is `cargo test --test perf_lane -- --ignored`.
//! The measurement is `#[ignore]`d, so an ordinary `cargo test` does not execute
//! it and shared-runner variance cannot make the normal suite flaky (NFR-003's
//! Verification section).
//!
//! A test target rather than a `harness = false` bench because a benchmark that
//! discharges a matrix row has to be a symbol a trace tag can bind: a bare `fn`
//! in a custom harness carries the tag and backs nothing
//! (agent-ix/quire-code-rs#8).

use std::collections::BTreeSet;
use std::time::Instant;

use quire_code_rs::{extract, Language, SourceFile};

/// The corpus NFR-003 states its budget against: 5,000 files, ~500,000 lines.
const TARGET_FILES: usize = 5_000;

/// NFR-003 thresholds.
const FULL_EXTRACTION_THRESHOLD_SECS: f64 = 60.0;
const SINGLE_FILE_P95_THRESHOLD_MS: f64 = 50.0;
const PEAK_MEMORY_THRESHOLD_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_FIXPOINT_ITERATIONS: usize = 10;

// TC-062, NFR-003-AC-1: the full benchmark corpus extracts within 60 s.
// TC-063, NFR-003-AC-2: single-file re-extraction p95 is within 50 ms.
// TC-064, NFR-003-AC-3: peak resident memory stays within 2.0 GB.
// TC-065, NFR-003-AC-4: the fixpoint converges within its ten-iteration bound.
// TC-070, NFR-004-AC-5: per-language recall is computed and reported.
//
// `#[ignore]` precedes `#[test]` deliberately: the ecosystem's Rust symbol
// scanner reads the attribute immediately above the `fn`, so the conventional
// order hides the symbol and every row above goes silently unbacked
// (agent-ix/quire-rs#387).
#[ignore = "benchmark lane: wall-clock budgets measure the host as much as the code, so they run under `make bench`, never in the PR gate"]
#[test]
fn extraction_meets_its_budgets_and_reports_its_recall() {
    let corpus = generate_corpus(TARGET_FILES);
    let total_lines: usize = corpus.iter().map(|f| f.content.lines().count()).sum();
    println!("corpus: {} files, {total_lines} lines\n", corpus.len());

    let (full, iterations, hit_bound) = bench_full_extraction(&corpus);
    let p95 = bench_single_file_reextraction(&corpus);
    let peak = peak_resident_bytes();
    let recall = corpus_recall(&corpus);

    println!("\n--- NFR-003 / NFR-004 ---");
    report(
        "TC-062 full extraction",
        format!("{full:.1} s"),
        full <= FULL_EXTRACTION_THRESHOLD_SECS,
        format!("<= {FULL_EXTRACTION_THRESHOLD_SECS:.0} s"),
    );
    report(
        "TC-063 single-file p95",
        format!("{p95:.1} ms"),
        p95 <= SINGLE_FILE_P95_THRESHOLD_MS,
        format!("<= {SINGLE_FILE_P95_THRESHOLD_MS:.0} ms"),
    );
    match peak {
        Some(bytes) => report(
            "TC-064 peak memory",
            format!("{:.2} GB", bytes as f64 / 1e9),
            bytes <= PEAK_MEMORY_THRESHOLD_BYTES,
            format!("<= {:.1} GB", PEAK_MEMORY_THRESHOLD_BYTES as f64 / 1e9),
        ),
        None => println!("TC-064 peak memory        : unavailable on this platform"),
    }
    report(
        "TC-065 fixpoint iterations",
        format!("{iterations} measured"),
        iterations as usize <= MAX_FIXPOINT_ITERATIONS && hit_bound == 0,
        format!("<= {MAX_FIXPOINT_ITERATIONS}"),
    );
    if hit_bound > 0 {
        eprintln!(
            "  {hit_bound} file(s) hit the iteration bound with bindings still \
             pending — edges were lost to the bound, not to the source"
        );
    }
    println!(
        "TC-070 corpus recall      : {:.2} ({}/{})  [reported, not gated]",
        recall.2, recall.0, recall.1
    );

    assert!(
        full <= FULL_EXTRACTION_THRESHOLD_SECS,
        "TC-062: full extraction took {full:.1} s, budget {FULL_EXTRACTION_THRESHOLD_SECS:.0} s"
    );
    assert!(
        p95 <= SINGLE_FILE_P95_THRESHOLD_MS,
        "TC-063: single-file p95 was {p95:.1} ms, budget {SINGLE_FILE_P95_THRESHOLD_MS:.0} ms"
    );
    // An unavailable measurement is not a passing one; TC-064 reports the
    // platform gap rather than counting it as within budget.
    if let Some(bytes) = peak {
        assert!(
            bytes <= PEAK_MEMORY_THRESHOLD_BYTES,
            "TC-064: peak RSS was {bytes} B, budget {PEAK_MEMORY_THRESHOLD_BYTES} B"
        );
    }
    assert!(
        iterations as usize <= MAX_FIXPOINT_ITERATIONS && hit_bound == 0,
        "TC-065: fixpoint took {iterations} iteration(s) with {hit_bound} file(s) still pending at the bound of {MAX_FIXPOINT_ITERATIONS}"
    );
    println!("\nAll NFR-003 thresholds met.");
}

fn report(label: &str, measured: String, ok: bool, threshold: String) {
    println!(
        "{label:<26}: {measured:<14} {threshold:<12} {}",
        if ok { "OK" } else { "FAIL" }
    );
}

/// Full extraction of the whole corpus (TC-062).
fn bench_full_extraction(corpus: &[SourceFile]) -> (f64, u32, u32) {
    let started = Instant::now();
    let result = extract(corpus);
    let elapsed = started.elapsed().as_secs_f64();
    println!(
        "full extraction: {} nodes, {} edges, {} unresolved calls in {elapsed:.1} s",
        result.nodes.len(),
        result.edges.len(),
        result.stats.unresolved_calls
    );
    (
        elapsed,
        result.stats.max_fixpoint_iterations,
        result.stats.files_hitting_iteration_bound,
    )
}

/// Re-extraction of a single file, p95 over a sample (TC-063).
fn bench_single_file_reextraction(corpus: &[SourceFile]) -> f64 {
    let sample: Vec<&SourceFile> = corpus.iter().step_by(corpus.len() / 200).collect();
    let mut timings: Vec<f64> = Vec::with_capacity(sample.len());

    for file in &sample {
        let batch = [(*file).clone()];
        let started = Instant::now();
        let _ = extract(&batch);
        timings.push(started.elapsed().as_secs_f64() * 1000.0);
    }

    timings.sort_by(|a, b| a.partial_cmp(b).expect("no NaN timings"));
    let index = ((timings.len() as f64) * 0.95).ceil() as usize - 1;
    let p95 = timings[index.min(timings.len() - 1)];
    println!(
        "single-file re-extraction over {} samples: p95 {p95:.1} ms",
        timings.len()
    );
    p95
}

/// Recall at corpus scale (TC-070): the generator knows every call it wrote.
fn corpus_recall(corpus: &[SourceFile]) -> (usize, usize, f64) {
    let known = known_bindings(corpus.len());
    let result = extract(corpus);
    let emitted: BTreeSet<(String, String)> = result
        .edges
        .iter()
        .filter(|e| e.edge_type == "calls")
        .map(|e| (e.source_ref.clone(), e.target_ref.clone()))
        .collect();

    // Precision is an absolute even here: nothing emitted may be outside the
    // known set (NFR-004).
    let wrong: Vec<_> = emitted.difference(&known).collect();
    assert!(
        wrong.is_empty(),
        "corpus extraction emitted {} call edge(s) outside the known-binding set: {:?}",
        wrong.len(),
        &wrong[..wrong.len().min(5)]
    );

    let recovered = emitted.intersection(&known).count();
    let recall = recovered as f64 / known.len() as f64;
    (recovered, known.len(), recall)
}

/// Peak resident set size in bytes (TC-064), where the platform reports it.
fn peak_resident_bytes() -> Option<u64> {
    // Linux reports it in /proc; macOS in `ru_maxrss` via `ps`. Reading either
    // is a benchmark-only concern — the library itself touches neither
    // (NFR-002).
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmHWM:") {
                let kb: u64 = rest.trim().trim_end_matches(" kB").trim().parse().ok()?;
                return Some(kb * 1024);
            }
        }
        None
    }
    #[cfg(target_os = "macos")]
    {
        let pid = std::process::id();
        let out = std::process::Command::new("ps")
            .args(["-o", "rss=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        let kb: u64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
        Some(kb * 1024)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

/// Every call binding the generator wrote, in emitted-edge form.
///
/// Each module `n` declares `TypeN` with `method_n`, and its `driver_n` calls
/// that method through a typed local — so the correct binding is known by
/// construction.
fn known_bindings(files: usize) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    for n in 0..files {
        let (org, repo) = ("agent-ix", "bench");
        let path = module_path(n);
        out.insert((
            format!("{org}/{repo}/{path}::driver_{n}"),
            format!("{org}/{repo}/{path}::Type{n}::method_{n}"),
        ));
        out.insert((
            format!("{org}/{repo}/{path}::Type{n}::chain_{n}"),
            format!("{org}/{repo}/{path}::Type{n}::method_{n}"),
        ));
    }
    out
}

fn module_path(n: usize) -> String {
    format!("src/pkg{}/mod{n}.rs", n / 100)
}

/// Generate the corpus deterministically: same size in, same bytes out.
fn generate_corpus(files: usize) -> Vec<SourceFile> {
    (0..files)
        .map(|n| {
            // ~100 lines per file, so 5,000 files is ~500,000 lines.
            let filler: String = (0..30)
                .map(|k| {
                    format!(
                        "    /// Helper {k} for module {n}. Implements FR-001.\n    \
                         pub fn helper_{n}_{k}(&self) -> u32 {{\n        {k}\n    }}\n"
                    )
                })
                .collect();

            let content = format!(
                r#"//! Generated benchmark module {n}. Implements FR-001 and NFR-003.

pub struct Type{n} {{
    pub count: u32,
}}

impl Type{n} {{
    pub fn method_{n}(&self) -> u32 {{
        self.count
    }}

    pub fn chain_{n}(&self) -> u32 {{
        self.method_{n}()
    }}

{filler}}}

/// Drives the type declared above. TC-062 exercises this shape at scale.
pub fn driver_{n}(value: Type{n}) -> u32 {{
    value.method_{n}()
}}
"#
            );
            SourceFile::new("agent-ix", "bench", module_path(n), Language::Rust, content)
        })
        .collect()
}
