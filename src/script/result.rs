/// Result types for script execution output.
use serde::Serialize;

/// Status of a single script step.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    /// Step executed successfully.
    Ok,
    /// Step failed.
    Error,
    /// Step was skipped (non-selected branch).
    Skipped,
}

/// Result entry for a single executed or skipped step.
#[derive(Debug, Clone, Serialize)]
pub struct StepResult {
    /// Zero-based position in the flattened execution trace.
    pub index: usize,
    /// The command argv (null for synthetic entries).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    /// Outcome of this step.
    pub status: StepStatus,
    /// Structured output from the underlying command (absent on skipped).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// Error details (present only when status == error).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Per-step wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Present only for iterations produced by a loop step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_index: Option<u64>,
}

/// The complete output of a script run, emitted as JSON to stdout.
#[derive(Debug, Serialize)]
pub struct RunReport {
    /// Ordered step results.
    pub results: Vec<StepResult>,
    /// Count of `ok` results.
    pub executed: usize,
    /// Count of `skipped` results.
    pub skipped: usize,
    /// Count of `error` results.
    pub failed: usize,
    /// Overall wall-clock duration in milliseconds.
    pub total_ms: u64,
}

/// The output of a `--dry-run` validation.
#[derive(Debug, Serialize)]
pub struct DryRunReport {
    /// Always `false` for dry-run (no CDP dispatch happened).
    pub dispatched: bool,
    /// Whether the script passed validation.
    pub ok: bool,
    /// Number of steps parsed.
    pub steps: usize,
}
