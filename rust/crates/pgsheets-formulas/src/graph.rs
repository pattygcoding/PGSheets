//! Maps cell references to their dependants, so an edit can be propagated to exactly the cells that
//! need recalculating.
//!
//! Two edge directions are tracked: `precedents` (a cell to the cells its formula reads) and
//! `dependents` (the reverse index, which is what an edit walks).

use std::collections::{HashMap, HashSet, VecDeque};

use pgsheets_core::CellAddress;

/// The dependency edges between cells.
#[derive(Default)]
pub struct DependencyGraph {
    precedents: HashMap<CellAddress, HashSet<CellAddress>>,
    dependents: HashMap<CellAddress, HashSet<CellAddress>>,
}

impl DependencyGraph {
    /// Creates an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Drops every edge.
    pub fn clear(&mut self) {
        self.precedents.clear();
        self.dependents.clear();
    }

    /// The cells a formula reads; `None` when it reads nothing.
    pub fn precedents(&self, cell: CellAddress) -> Option<&HashSet<CellAddress>> {
        self.precedents.get(&cell)
    }

    /// Replaces the outgoing edges of `cell`.
    ///
    /// Called every time a formula is evaluated, which is what keeps the graph correct as formulas are
    /// edited: the old edges are detached and the new ones attached.
    pub fn set_precedents(&mut self, cell: CellAddress, precedents: &HashSet<CellAddress>) {
        if let Some(existing) = self.precedents.get(&cell) {
            for precedent in existing.clone() {
                if let Some(dependents) = self.dependents.get_mut(&precedent) {
                    dependents.remove(&cell);

                    if dependents.is_empty() {
                        self.dependents.remove(&precedent);
                    }
                }
            }

            if let Some(existing) = self.precedents.get_mut(&cell) {
                existing.clear();
            }
        }

        if precedents.is_empty() {
            self.precedents.remove(&cell);
            return;
        }

        let target = self.precedents.entry(cell).or_default();

        for precedent in precedents {
            if !target.insert(*precedent) {
                continue;
            }

            self.dependents.entry(*precedent).or_default().insert(cell);
        }
    }

    /// Detaches a deleted cell's outgoing edges.
    ///
    /// Incoming edges are deliberately kept, because other cells may still reference the address: they
    /// will see it as empty and must recalculate.
    pub fn remove_cell(&mut self, cell: CellAddress) {
        let Some(precedents) = self.precedents.remove(&cell) else {
            return;
        };

        for precedent in precedents {
            if let Some(dependents) = self.dependents.get_mut(&precedent) {
                dependents.remove(&cell);

                if dependents.is_empty() {
                    self.dependents.remove(&precedent);
                }
            }
        }
    }

    /// Collects `start` plus everything reachable through dependent edges: the cells affected by an
    /// edit.
    pub fn collect_affected(&self, start: CellAddress, into: &mut HashSet<CellAddress>) {
        into.insert(start);

        let mut queue = VecDeque::new();
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            let Some(dependents) = self.dependents.get(&current) else {
                continue;
            };

            for dependent in dependents {
                if into.insert(*dependent) {
                    queue.push_back(*dependent);
                }
            }
        }
    }

    /// Number of cells with outgoing edges, for tests and diagnostics.
    pub fn edge_count(&self) -> usize {
        self.precedents.values().map(HashSet::len).sum()
    }
}
