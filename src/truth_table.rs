//! The truth table: the result of evaluating one proposition
//! (`design/truth_table.md`).
//!
//! The [`Interpreter`](crate::interpreter::Interpreter) creates a table with
//! its variables and rows; the Orchestrator adds the display text through
//! [`TruthTable::with_display`]; the UI presents it. The table is immutable end
//! to end (ADR-020), and its fields are public plain data, like
//! [`Expression`](crate::expression::Expression)'s (ADR-040).

/// One truth table: the ordered variables, the rows, and the display text of
/// the evaluated and (optionally) simplified formula.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TruthTable {
    /// One column per distinct variable of the evaluated expression, in order
    /// of first appearance.
    pub variables: Vec<String>,
    /// One row per truth assignment, in binary counting order with the last
    /// variable varying fastest.
    pub rows: Vec<Row>,
    /// The label of the result column: the evaluated formula in normalized
    /// infix form. Empty until the Orchestrator calls
    /// [`with_display`](TruthTable::with_display).
    pub label: String,
    /// The simplified formula in canonical order, present only when the
    /// Simplifier ran and the Orchestrator set it.
    pub simplified: Option<String>,
}

/// One truth assignment: the variables' values in column order, and the value
/// the proposition takes under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The value of each variable, in the table's column order.
    pub values: Vec<bool>,
    /// The value of the evaluated expression under this assignment.
    pub result: bool,
}

/// The class of a proposition, read from the result column (ADR-021).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    /// Every value in the result column is true.
    Tautology,
    /// Every value in the result column is false.
    Contradiction,
    /// Neither: the proposition is true under some assignments and false under
    /// others.
    Contingency,
}

impl TruthTable {
    /// Returns a copy of the table with the two display fields set. There are
    /// no setters and no mutation (ADR-020).
    pub fn with_display(&self, label: String, simplified: Option<String>) -> TruthTable {
        TruthTable {
            variables: self.variables.clone(),
            rows: self.rows.clone(),
            label,
            simplified,
        }
    }

    /// The class of the proposition, derived from the result column. It reads
    /// the column, not a simplified formula, so it is exact (ADR-021).
    ///
    /// A table always has at least one row (2^0 = 1), so the two "all" cases
    /// cannot both hold; an empty table is only constructible by hand and
    /// classifies as a contingency.
    pub fn classify(&self) -> Classification {
        if self.rows.is_empty() {
            return Classification::Contingency;
        }
        if self.rows.iter().all(|row| row.result) {
            Classification::Tautology
        } else if self.rows.iter().all(|row| !row.result) {
            Classification::Contradiction
        } else {
            Classification::Contingency
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Classification, TruthTable};
    use crate::expression::Expression;
    use crate::interpreter::Interpreter;

    fn parse(source: &str) -> Expression {
        let tokens = crate::scanner::scan(source.to_string()).expect("scan");
        let mut expressions = crate::parser::Parser::parse(tokens).expect("parse");
        assert_eq!(expressions.len(), 1, "source {source:?}");
        expressions.remove(0)
    }

    fn table(source: &str) -> TruthTable {
        Interpreter::interpret(&parse(source)).expect("the expression should evaluate")
    }

    // --- Classification ----------------------------------------------------

    #[test]
    fn a_proposition_true_under_every_assignment_is_a_tautology() {
        assert_eq!(table("p | !p").classify(), Classification::Tautology);
    }

    #[test]
    fn a_proposition_false_under_every_assignment_is_a_contradiction() {
        assert_eq!(table("p & !p").classify(), Classification::Contradiction);
    }

    #[test]
    fn a_single_variable_is_a_contingency() {
        assert_eq!(table("p").classify(), Classification::Contingency);
    }

    #[test]
    fn an_implication_is_a_contingency() {
        assert_eq!(table("p -> q").classify(), Classification::Contingency);
    }

    // --- with_display ------------------------------------------------------

    #[test]
    fn with_display_sets_the_label_and_simplified_text() {
        let base = table("p & q");

        let displayed = base.with_display("p ∧ q".to_string(), Some("p ∧ q".to_string()));

        assert_eq!(displayed.label, "p ∧ q");
        assert_eq!(displayed.simplified, Some("p ∧ q".to_string()));
        assert_eq!(displayed.variables, base.variables);
        assert_eq!(displayed.rows, base.rows);
    }

    #[test]
    fn with_display_accepts_a_table_without_simplified_text() {
        let base = table("p & q");

        let displayed = base.with_display("p ∧ q".to_string(), None);

        assert_eq!(displayed.label, "p ∧ q");
        assert_eq!(displayed.simplified, None);
    }

    #[test]
    fn with_display_does_not_modify_the_original_table() {
        let base = table("p & q");
        let before = base.clone();

        let _displayed = base.with_display("p ∧ q".to_string(), Some("p ∧ q".to_string()));

        assert_eq!(base, before);
        assert_eq!(base.label, "");
        assert_eq!(base.simplified, None);
    }
}
