// tests/test_harness.rs
//! Test harness for tracking YAML 1.2.2 compliance progress.
//!
//! This module provides utilities to:
//! - Track test pass/fail counts by chapter
//! - Compare against baseline to detect regressions
//! - Generate compliance reports

use std::collections::HashMap;

/// Test result statistics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestStats {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
}

impl TestStats {
    pub fn new() -> Self {
        Self {
            total: 0,
            passed: 0,
            failed: 0,
            ignored: 0,
        }
    }

    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.passed as f64 / self.total as f64) * 100.0
        }
    }
}

/// Compliance report by chapter
#[derive(Debug, Clone)]
pub struct ComplianceReport {
    pub chapters: HashMap<usize, TestStats>,
    pub overall: TestStats,
    pub implementation: TestStats,
}

impl ComplianceReport {
    pub fn new() -> Self {
        Self {
            chapters: HashMap::new(),
            overall: TestStats::new(),
            implementation: TestStats::new(),
        }
    }

    /// Record a test result
    pub fn record_test(&mut self, chapter: usize, passed: bool, ignored: bool) {
        let stats = self.chapters.entry(chapter).or_insert_with(TestStats::new);
        stats.total += 1;
        self.overall.total += 1;

        if ignored {
            stats.ignored += 1;
            self.overall.ignored += 1;
        } else if passed {
            stats.passed += 1;
            self.overall.passed += 1;
        } else {
            stats.failed += 1;
            self.overall.failed += 1;
        }
    }

    /// Record an implementation test result
    pub fn record_impl_test(&mut self, passed: bool) {
        self.implementation.total += 1;
        if passed {
            self.implementation.passed += 1;
        } else {
            self.implementation.failed += 1;
        }
    }

    /// Print a formatted report
    pub fn print_report(&self) {
        println!("\n{}", "=".repeat(70));
        println!("YAML 1.2.2 COMPLIANCE REPORT");
        println!("{}", "=".repeat(70));

        // Overall stats
        println!(
            "\nOVERALL: {}/{} tests passing ({:.1}%)",
            self.overall.passed,
            self.overall.total,
            self.overall.pass_rate()
        );
        if self.overall.ignored > 0 {
            println!("  Ignored: {}", self.overall.ignored);
        }

        // By chapter
        println!("\nBY CHAPTER:");
        let mut chapters: Vec<_> = self.chapters.iter().collect();
        chapters.sort_by_key(|(ch, _)| **ch);

        for (chapter, stats) in chapters {
            let status = if stats.failed == 0 {
                "✅"
            } else if stats.pass_rate() >= 90.0 {
                "⚡"
            } else {
                "⚠️ "
            };

            println!(
                "  Ch {}: {} {}/{} passing ({:.1}%)",
                chapter,
                status,
                stats.passed,
                stats.total,
                stats.pass_rate()
            );

            if stats.failed > 0 {
                println!("      {} failures", stats.failed);
            }
        }

        // Implementation tests
        if self.implementation.total > 0 {
            println!("\nIMPLEMENTATION TESTS:");
            println!(
                "  {}/{} passing ({:.1}%)",
                self.implementation.passed,
                self.implementation.total,
                self.implementation.pass_rate()
            );
        }

        println!("{}", "=".repeat(70));
    }

    /// Check if this report shows regression from baseline
    pub fn check_regression(&self, baseline_passed: usize) -> Result<(), String> {
        if self.overall.passed < baseline_passed {
            Err(format!(
                "REGRESSION DETECTED: Only {} tests passing (baseline: {})",
                self.overall.passed, baseline_passed
            ))
        } else if self.overall.passed > baseline_passed {
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Export as JSON for tracking
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\n");
        json.push_str(&format!("  \"overall\": {{\n"));
        json.push_str(&format!("    \"total\": {},\n", self.overall.total));
        json.push_str(&format!("    \"passed\": {},\n", self.overall.passed));
        json.push_str(&format!("    \"failed\": {},\n", self.overall.failed));
        json.push_str(&format!("    \"ignored\": {},\n", self.overall.ignored));
        json.push_str(&format!(
            "    \"pass_rate\": {:.2}\n",
            self.overall.pass_rate()
        ));
        json.push_str("  },\n");

        json.push_str("  \"chapters\": {\n");
        let mut chapters: Vec<_> = self.chapters.iter().collect();
        chapters.sort_by_key(|(ch, _)| **ch);

        for (i, (chapter, stats)) in chapters.iter().enumerate() {
            json.push_str(&format!("    \"{}\": {{\n", chapter));
            json.push_str(&format!("      \"total\": {},\n", stats.total));
            json.push_str(&format!("      \"passed\": {},\n", stats.passed));
            json.push_str(&format!("      \"failed\": {},\n", stats.failed));
            json.push_str(&format!("      \"ignored\": {},\n", stats.ignored));
            json.push_str(&format!("      \"pass_rate\": {:.2}\n", stats.pass_rate()));
            if i < chapters.len() - 1 {
                json.push_str("    },\n");
            } else {
                json.push_str("    }\n");
            }
        }
        json.push_str("  },\n");

        json.push_str("  \"implementation\": {\n");
        json.push_str(&format!("    \"total\": {},\n", self.implementation.total));
        json.push_str(&format!("    \"passed\": {},\n", self.implementation.passed));
        json.push_str(&format!("    \"failed\": {},\n", self.implementation.failed));
        json.push_str(&format!(
            "    \"pass_rate\": {:.2}\n",
            self.implementation.pass_rate()
        ));
        json.push_str("  }\n");

        json.push_str("}\n");
        json
    }
}

/// Parse chapter number from test name
/// Examples: ch_2_3_01_literal_block_scalar -> Some(2)
pub fn extract_chapter(test_name: &str) -> Option<usize> {
    if !test_name.starts_with("ch_") {
        return None;
    }

    let parts: Vec<&str> = test_name.split('_').collect();
    if parts.len() < 2 {
        return None;
    }

    parts[1].parse::<usize>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_chapter() {
        assert_eq!(extract_chapter("ch_2_3_01_literal_block_scalar"), Some(2));
        assert_eq!(extract_chapter("ch_10_2_04_json_integer"), Some(10));
        assert_eq!(extract_chapter("some_other_test"), None);
    }

    #[test]
    fn test_stats_pass_rate() {
        let mut stats = TestStats::new();
        stats.total = 100;
        stats.passed = 90;
        stats.failed = 10;

        assert_eq!(stats.pass_rate(), 90.0);
    }

    #[test]
    fn test_report_regression_check() {
        let mut report = ComplianceReport::new();
        report.overall.total = 100;
        report.overall.passed = 90;

        // No regression if we're at baseline
        assert!(report.check_regression(90).is_ok());

        // No regression if we improved
        assert!(report.check_regression(85).is_ok());

        // Regression if we dropped
        assert!(report.check_regression(95).is_err());
    }
}
