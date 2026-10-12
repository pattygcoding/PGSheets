//! The default formula engine.
//!
//! Evaluation is a memoised depth-first walk: the changed cell is evaluated, which recursively
//! evaluates whatever it references, while a set of "in progress" cells detects circular references.
//! Results are written back to the store as they are produced, and the dependency graph records the
//! edges discovered along the way.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use pgsheets_core::store::CellStore;
use pgsheets_core::{CellAddress, CellError, CellValue, FormulaEngine, parse_literal};

use crate::ast::{AstNode, FormulaContext};
use crate::functions::FunctionLibrary;
use crate::graph::DependencyGraph;
use crate::parser;

/// The default engine: a parsed-formula cache, a dependency graph, and the memoised walk.
///
/// The store it reads and writes is passed to each call rather than held. That is the one structural
/// difference from the C# original — a Rust field cannot borrow its owner — and it is also what lets
/// the engine own nothing but its own caches.
pub struct Engine {
    graph: DependencyGraph,

    /// Parsed formulas, by cell. Parsing only ever happens after an edit, never per frame; an
    /// unparseable formula is cached as an error node so it is not re-parsed on every recalculation.
    parse_cache: HashMap<CellAddress, Rc<AstNode>>,

    functions: FunctionLibrary,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Creates an engine with the standard function library.
    pub fn new() -> Self {
        Self::with_library(FunctionLibrary::standard())
    }

    /// Creates an engine over a library, so a host can extend or replace the function set.
    pub fn with_library(functions: FunctionLibrary) -> Self {
        Self {
            graph: DependencyGraph::new(),
            parse_cache: HashMap::new(),
            functions,
        }
    }

    /// The library calls are resolved against.
    pub fn functions(&self) -> &FunctionLibrary {
        &self.functions
    }

    /// The dependency edges discovered so far.
    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }

    /// Recalculates the changed cell and everything that depends on it.
    pub fn recalculate(&mut self, store: &mut dyn CellStore, changed: CellAddress) {
        // The edited cell's formula text changed, so its cached parse tree is stale.
        self.parse_cache.remove(&changed);

        let mut affected = HashSet::new();
        self.graph.collect_affected(changed, &mut affected);

        let mut context = EvalContext::new(self, store);

        for cell in affected {
            context.evaluate(cell);
        }
    }

    /// Recomputes the entire workbook. Used once after a bulk import.
    pub fn rebuild(&mut self, store: &mut dyn CellStore) {
        self.graph.clear();
        self.parse_cache.clear();

        let cells = store.populated_cells();
        let mut context = EvalContext::new(self, store);

        for cell in cells {
            context.evaluate(cell);
        }
    }

    /// Drops all cached state: parse trees and the dependency graph.
    pub fn reset(&mut self) {
        self.graph.clear();
        self.parse_cache.clear();
    }

    /// Notifies the engine that a cell was deleted, so its edges can be detached.
    pub fn on_cell_removed(&mut self, cell: CellAddress) {
        self.graph.remove_cell(cell);
        self.parse_cache.remove(&cell);
    }

    /// The cells a formula directly reads.
    pub fn precedents(&self, cell: CellAddress) -> Vec<CellAddress> {
        self.graph
            .precedents(cell)
            .map(|precedents| precedents.iter().copied().collect())
            .unwrap_or_default()
    }
}

impl FormulaEngine for Engine {
    fn recalculate(&mut self, store: &mut dyn CellStore, changed: CellAddress) {
        Engine::recalculate(self, store, changed);
    }

    fn rebuild(&mut self, store: &mut dyn CellStore) {
        Engine::rebuild(self, store);
    }

    fn reset(&mut self) {
        Engine::reset(self);
    }

    fn on_cell_removed(&mut self, cell: CellAddress) {
        Engine::on_cell_removed(self, cell);
    }

    fn precedents(&self, cell: CellAddress) -> Vec<CellAddress> {
        Engine::precedents(self, cell)
    }
}

/// Per-run evaluation state.
///
/// It borrows the engine and the store together for the length of one recalculation, which is what
/// makes the recursion possible without any shared mutable state: the caches live in the engine, the
/// cells live in the store, and neither is reachable from anywhere else while this exists.
struct EvalContext<'a> {
    engine: &'a mut Engine,
    store: &'a mut dyn CellStore,
    in_progress: HashSet<CellAddress>,
    done: HashSet<CellAddress>,
}

impl<'a> EvalContext<'a> {
    fn new(engine: &'a mut Engine, store: &'a mut dyn CellStore) -> Self {
        Self {
            engine,
            store,
            in_progress: HashSet::new(),
            done: HashSet::new(),
        }
    }

    /// Evaluates a cell, recursively evaluating whatever it depends on.
    fn evaluate(&mut self, cell: CellAddress) -> CellValue {
        if self.done.contains(&cell) {
            return self.store.value(cell.row, cell.column);
        }

        if !self.in_progress.insert(cell) {
            // Re-entering a cell that is still being evaluated means the formulas form a cycle.
            return CellValue::error(CellError::Circular);
        }

        let raw = self.store.raw_input(cell.row, cell.column).to_string();

        let value = if raw.is_empty() {
            CellValue::Empty
        } else if raw.starts_with('=') {
            self.evaluate_formula(cell, &raw)
        } else {
            parse_literal(&raw)
        };

        self.in_progress.remove(&cell);
        self.done.insert(cell);

        // A cell that does not exist is ignored by the store rather than recreated, which is what keeps
        // a deleted cell deleted.
        self.store.set_computed(cell.row, cell.column, value.clone());

        value
    }

    fn evaluate_formula(&mut self, cell: CellAddress, raw: &str) -> CellValue {
        let node = match self.engine.parse_cache.get(&cell) {
            // A reference-counted clone, so the cache is not borrowed while the formula is evaluated.
            Some(node) => Rc::clone(node),
            None => {
                let node = Rc::new(parser::parse_or_error_node(raw));
                self.engine.parse_cache.insert(cell, Rc::clone(&node));
                node
            }
        };

        let mut precedents = HashSet::new();
        node.collect_precedents(&mut precedents);
        self.engine.graph.set_precedents(cell, &precedents);

        node.evaluate(self)
    }
}

impl FormulaContext for EvalContext<'_> {
    fn cell_value(&mut self, row: i32, column: i32) -> CellValue {
        self.evaluate(CellAddress::new(row, column))
    }

    fn functions(&self) -> &FunctionLibrary {
        self.engine.functions()
    }
}
