//! The pipeline driver (`design/orchestrator.md`).
//!
//! The book's `Lox.run` pattern, in functional form: `evaluate` composes the
//! stages source → tokens → expressions → simplified expressions → truth
//! tables, and `report` formats one error for the UI. Both are static and
//! pure; only the CLI performs I/O.
//!
//! Stages gate each other: a scan failure stops before parsing, a parse failure
//! stops before simplification, and an eval failure aborts the whole run, so an
//! `Err` list only ever holds errors of the stage that failed (ADR-002,
//! ADR-019).

use crate::error::Error;
use crate::expression::InfixPrinter;
use crate::interpreter::Interpreter;
use crate::parser::Parser;
use crate::scanner::scan;
use crate::simplifier::Simplifier;
use crate::truth_table::TruthTable;

/// The pipeline driver. Never constructed.
pub struct Orchestrator;

impl Orchestrator {
    /// Runs the whole pipeline over `source` and returns one truth table per
    /// proposition, in input order.
    ///
    /// When `simplify` is false the Simplifier is skipped and the original is
    /// tabulated. When it is true, the original is tabulated by default and
    /// the simplified expression with `table_simplified`; either way the
    /// simplified formula is kept as the table's display text. With
    /// `simplify = false` the `table_simplified` flag has no effect (ADR-026).
    pub fn evaluate(
        source: String,
        simplify: bool,
        table_simplified: bool,
    ) -> Result<Vec<TruthTable>, Vec<Error>> {
        let tokens = scan(source)?;
        let expressions = Parser::parse(tokens)?;

        let mut tables = Vec::with_capacity(expressions.len());
        for expression in &expressions {
            let simplified = simplify.then(|| Simplifier::simplify(expression));
            let chosen = match &simplified {
                Some(simplified) if table_simplified => simplified,
                _ => expression,
            };
            let table = Interpreter::interpret(chosen)?;
            let label = InfixPrinter::print(chosen);
            let simplified = simplified.as_ref().map(InfixPrinter::print);
            tables.push(table.with_display(label, simplified));
        }
        Ok(tables)
    }

    /// Formats one error exactly as the user reads it (ADR-037).
    pub fn report(error: Error) -> String {
        error.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::Orchestrator;
    use crate::error::Error;
    use crate::truth_table::{Classification, TruthTable};

    fn evaluate(
        source: &str,
        simplify: bool,
        table_simplified: bool,
    ) -> Result<Vec<TruthTable>, Vec<Error>> {
        Orchestrator::evaluate(source.to_string(), simplify, table_simplified)
    }

    fn one_table(source: &str, simplify: bool, table_simplified: bool) -> TruthTable {
        let mut tables = evaluate(source, simplify, table_simplified)
            .unwrap_or_else(|errors| panic!("source {source:?} failed: {errors:?}"));
        assert_eq!(tables.len(), 1, "source {source:?}");
        tables.remove(0)
    }

    /// `p1 | !p1 | p2 | ... | p13`: a tautology with thirteen variables, which
    /// the default table refuses and the simplified one collapses to `V`.
    fn tautology_with_thirteen_variables() -> String {
        let mut source = String::from("p1 | !p1");
        for index in 2..=13 {
            source.push_str(&format!(" | p{index}"));
        }
        source
    }

    fn results(table: &TruthTable) -> Vec<bool> {
        table.rows.iter().map(|row| row.result).collect()
    }

    // --- Default table -----------------------------------------------------

    #[test]
    fn the_default_table_uses_the_original_for_columns_rows_and_label() {
        let table = one_table("p -> q", true, false);

        assert_eq!(table.variables, ["p", "q"]);
        assert_eq!(table.label, "p → q");
        assert_eq!(table.simplified, Some("q ∨ ¬p".to_string()));
        assert_eq!(
            table
                .rows
                .iter()
                .map(|row| (row.values.clone(), row.result))
                .collect::<Vec<(Vec<bool>, bool)>>(),
            [
                (vec![false, false], true),
                (vec![false, true], true),
                (vec![true, false], false),
                (vec![true, true], true),
            ]
        );
    }

    // --- Table of the simplified expression --------------------------------

    #[test]
    fn table_simplified_uses_the_simplified_expression_for_columns_and_label() {
        let table = one_table("p -> q", true, true);

        assert_eq!(table.variables, ["q", "p"]);
        assert_eq!(table.label, "q ∨ ¬p");
        assert_eq!(table.simplified, Some("q ∨ ¬p".to_string()));
        assert_eq!(results(&table), [true, false, true, true]);
    }

    // --- Tautologies -------------------------------------------------------

    #[test]
    fn a_tautology_default_table_keeps_its_variable_column_and_reports_v() {
        let table = one_table("p | !p", true, false);

        assert_eq!(table.variables, ["p"]);
        assert_eq!(table.rows.len(), 2);
        assert!(table.rows.iter().all(|row| row.result));
        assert_eq!(table.label, "p ∨ ¬p");
        assert_eq!(table.simplified, Some("V".to_string()));
        assert_eq!(table.classify(), Classification::Tautology);
    }

    #[test]
    fn a_tautology_simplified_table_has_no_variables_and_one_true_row() {
        let table = one_table("p | !p", true, true);

        assert!(table.variables.is_empty());
        assert_eq!(table.rows.len(), 1);
        assert!(table.rows[0].values.is_empty());
        assert!(table.rows[0].result);
        assert_eq!(table.label, "V");
        assert_eq!(table.simplified, Some("V".to_string()));
    }

    // --- Size limit on the chosen expression -------------------------------

    #[test]
    fn a_thirteen_variable_original_is_refused_by_the_default_table() {
        let errors = evaluate(&tautology_with_thirteen_variables(), true, false)
            .expect_err("thirteen variables must exceed the limit");

        assert_eq!(
            errors,
            [Error::EvalError {
                message: "resultado muy largo: más de 4096 filas".to_string(),
            }]
        );
    }

    #[test]
    fn a_thirteen_variable_original_succeeds_when_the_simplified_table_is_chosen() {
        let table = one_table(&tautology_with_thirteen_variables(), true, true);

        assert!(table.variables.is_empty());
        assert_eq!(table.rows.len(), 1);
        assert!(table.rows[0].result);
        assert_eq!(table.label, "V");
    }

    // --- Simplifier skipping ----------------------------------------------

    #[test]
    fn simplify_false_skips_the_simplifier_and_keeps_the_original_table() {
        let table = one_table("p -> q", false, false);
        let simplified = one_table("p -> q", true, false);

        assert_eq!(table.simplified, None);
        assert_eq!(table.label, "p → q");
        assert_eq!(table.variables, simplified.variables);
        assert_eq!(table.rows, simplified.rows);
    }

    #[test]
    fn simplify_false_ignores_the_table_simplified_flag() {
        let without_flag = one_table("p -> q", false, false);
        let with_flag = one_table("p -> q", false, true);

        assert_eq!(with_flag, without_flag);
        assert_eq!(with_flag.simplified, None);
    }

    // --- Stage gates -------------------------------------------------------

    #[test]
    fn a_scan_error_yields_only_scan_errors_and_no_tables() {
        let errors = evaluate("p % q", true, false).expect_err("the source must not scan");

        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .all(|error| matches!(error, Error::ScanError { .. })),
            "mixed error kinds: {errors:?}"
        );
    }

    #[test]
    fn a_parse_error_yields_only_parse_errors_and_no_tables() {
        let errors = evaluate("p &", true, false).expect_err("the source must not parse");

        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .all(|error| matches!(error, Error::ParseError { .. })),
            "mixed error kinds: {errors:?}"
        );
    }

    #[test]
    fn an_eval_error_yields_only_eval_errors_and_no_tables() {
        let errors = evaluate(&tautology_with_thirteen_variables(), true, false)
            .expect_err("the table exceeds the limit");

        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .all(|error| matches!(error, Error::EvalError { .. })),
            "mixed error kinds: {errors:?}"
        );
    }

    // --- Batch order -------------------------------------------------------

    #[test]
    fn a_batch_returns_one_table_per_proposition_in_input_order() {
        let tables =
            evaluate("p & q\nq & p\nr\n", false, false).expect("the batch should evaluate");

        assert_eq!(tables.len(), 3);
        assert_eq!(tables[0].variables, ["p", "q"]);
        assert_eq!(tables[0].label, "p ∧ q");
        assert_eq!(tables[1].variables, ["q", "p"]);
        assert_eq!(tables[1].label, "q ∧ p");
        assert_eq!(tables[2].variables, ["r"]);
        assert_eq!(tables[2].label, "r");
    }

    #[test]
    fn an_empty_source_returns_no_tables() {
        assert_eq!(evaluate("", true, false), Ok(vec![]));
        assert_eq!(evaluate("  \n\n", true, false), Ok(vec![]));
    }

    // --- Unchanged simplified text ----------------------------------------

    #[test]
    fn a_simplified_text_is_present_even_when_the_formula_does_not_change() {
        let table = one_table("p", true, false);

        assert_eq!(table.label, "p");
        assert_eq!(table.simplified, Some("p".to_string()));
    }

    // --- report ------------------------------------------------------------

    #[test]
    fn report_formats_a_scan_error_with_its_position() {
        let error = Error::ScanError {
            line: 2,
            col: 7,
            message: "carácter inválido '%'".to_string(),
        };

        assert_eq!(
            Orchestrator::report(error),
            "línea 2, columna 7: carácter inválido '%'"
        );
    }

    #[test]
    fn report_formats_a_parse_error_with_its_position() {
        let error = Error::ParseError {
            line: 1,
            col: 4,
            message: "se esperaba una expresión".to_string(),
        };

        assert_eq!(
            Orchestrator::report(error),
            "línea 1, columna 4: se esperaba una expresión"
        );
    }

    #[test]
    fn report_formats_an_eval_error_as_its_bare_message() {
        let error = Error::EvalError {
            message: "resultado muy largo: más de 4096 filas".to_string(),
        };

        assert_eq!(
            Orchestrator::report(error),
            "resultado muy largo: más de 4096 filas"
        );
    }
}
