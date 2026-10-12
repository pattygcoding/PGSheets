//! The formula engine: parser, AST, dependency graph, function library, writer and rewriter.
//!
//! Formulas are stored as text and are the source of truth; this crate is the only thing that reads
//! them, and it never rewrites the stored text except through the rewriter, which leaves a formula it
//! cannot parse exactly as it was.

pub mod ast;
pub mod engine;
pub mod functions;
pub mod graph;
pub mod parser;
pub mod rewriter;
pub mod writer;

pub use ast::{AstNode, CellRangeRef, FormulaContext};
pub use engine::Engine;
pub use functions::{BuiltinFn, FunctionLibrary};
pub use graph::DependencyGraph;
pub use parser::{FormulaError, parse, parse_or_error_node};
pub use writer::write;
