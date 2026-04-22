/// Batch script execution module.
///
/// Reads a JSON script file, executes each step against an existing CDP session,
/// and emits a structured JSON result array. Supports conditional branching, loops,
/// and variable binding.
pub mod context;
pub mod dispatch;
pub mod eval;
pub mod parser;
pub mod result;
pub mod runner;
