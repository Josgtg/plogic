//! The pipeline driver (`design/orchestrator.md`).
//!
//! The book's `Lox.run` pattern, in functional form: `evaluate` composes the
//! stages source → tokens → expressions → simplified expressions → truth
//! tables, `simplify` stops at the simplified formulas, and `report` formats
//! one error for the UI. All three are static and pure; only the CLI performs
//! I/O.
//!
//! Stages gate each other: a scan failure stops before parsing, a parse failure
//! stops before simplification, and an eval failure aborts the whole run, so an
//! `Err` list only ever holds errors of the stage that failed (ADR-002,
//! ADR-019).

use crate::error::Error;
use crate::expression::{Expression, InfixPrinter};
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
    /// tabulated. When it is true the table is built from the simplified
    /// expression, and the simplified formula is kept as the table's display
    /// text.
    pub fn evaluate(source: String, simplify: bool) -> Result<Vec<TruthTable>, Vec<Error>> {
        let expressions = Self::parse(source)?;

        let mut tables = Vec::with_capacity(expressions.len());
        for expression in &expressions {
            let simplified = simplify.then(|| Simplifier::simplify(expression));
            let chosen = simplified.as_ref().unwrap_or(expression);
            let table = Interpreter::interpret(chosen)?;
            let label = InfixPrinter::print(chosen);
            let simplified = simplified.as_ref().map(InfixPrinter::print);
            tables.push(table.with_display(label, simplified));
        }
        Ok(tables)
    }

    /// Simplifies every proposition of `source` and returns their normalized
    /// infix text, in input order. No table is built, so the size limit does
    /// not apply (`--simplificar`).
    pub fn simplify(source: String) -> Result<Vec<String>, Vec<Error>> {
        let expressions = Self::parse(source)?;
        Ok(expressions
            .iter()
            .map(|expression| InfixPrinter::print(&Simplifier::simplify(expression)))
            .collect())
    }

    /// The shared first half of both entry points: source to expressions.
    fn parse(source: String) -> Result<Vec<Expression>, Vec<Error>> {
        let tokens = scan(source)?;
        Parser::parse(tokens)
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

    fn evaluate(source: &str, simplify: bool) -> Result<Vec<TruthTable>, Vec<Error>> {
        Orchestrator::evaluate(source.to_string(), simplify)
    }

    fn formulas(source: &str) -> Result<Vec<String>, Vec<Error>> {
        Orchestrator::simplify(source.to_string())
    }

    fn one_table(source: &str, simplify: bool) -> TruthTable {
        let mut tables = evaluate(source, simplify)
            .unwrap_or_else(|errors| panic!("source {source:?} failed: {errors:?}"));
        assert_eq!(tables.len(), 1, "source {source:?}");
        tables.remove(0)
    }

    /// `p1 | !p1 | p2 | ... | p13`: a tautology with thirteen variables, which
    /// the original table refuses and the simplified one collapses to `V`.
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
    fn the_default_table_uses_the_simplified_expression_for_columns_rows_and_label() {
        let table = one_table("p -> q", true);

        assert_eq!(table.variables, ["q", "p"]);
        assert_eq!(table.label, "q ∨ ¬p");
        assert_eq!(table.simplified, Some("q ∨ ¬p".to_string()));
        assert_eq!(
            table
                .rows
                .iter()
                .map(|row| (row.values.clone(), row.result))
                .collect::<Vec<(Vec<bool>, bool)>>(),
            [
                (vec![false, false], true),
                (vec![false, true], false),
                (vec![true, false], true),
                (vec![true, true], true),
            ]
        );
    }

    // --- Original table ----------------------------------------------------

    #[test]
    fn simplify_false_uses_the_original_for_columns_rows_and_label() {
        let table = one_table("p -> q", false);

        assert_eq!(table.variables, ["p", "q"]);
        assert_eq!(table.label, "p → q");
        assert_eq!(table.simplified, None);
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

    // --- Tautologies -------------------------------------------------------

    #[test]
    fn a_tautology_default_table_has_no_variables_and_one_true_row() {
        let table = one_table("p | !p", true);

        assert!(table.variables.is_empty());
        assert_eq!(table.rows.len(), 1);
        assert!(table.rows[0].values.is_empty());
        assert!(table.rows[0].result);
        assert_eq!(table.label, "V");
        assert_eq!(table.simplified, Some("V".to_string()));
        assert_eq!(table.classify(), Classification::Tautology);
    }

    #[test]
    fn a_tautology_with_simplify_false_keeps_the_variable_column() {
        let table = one_table("p | !p", false);

        assert_eq!(table.variables, ["p"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].values, [false]);
        assert_eq!(table.rows[1].values, [true]);
        assert_eq!(results(&table), [true, true]);
        assert_eq!(table.label, "p ∨ ¬p");
        assert_eq!(table.simplified, None);
        assert_eq!(table.classify(), Classification::Tautology);
    }

    // --- Size limit on the chosen expression -------------------------------

    #[test]
    fn a_thirteen_variable_original_succeeds_by_default_after_simplification() {
        let table = one_table(&tautology_with_thirteen_variables(), true);

        assert!(table.variables.is_empty());
        assert_eq!(table.rows.len(), 1);
        assert!(table.rows[0].result);
        assert_eq!(table.label, "V");
        assert_eq!(table.simplified, Some("V".to_string()));
    }

    #[test]
    fn a_thirteen_variable_original_is_refused_with_simplify_false() {
        let errors = evaluate(&tautology_with_thirteen_variables(), false)
            .expect_err("thirteen variables must exceed the limit");

        assert_eq!(
            errors,
            [Error::EvalError {
                message: "resultado muy largo: más de 4096 filas".to_string(),
            }]
        );
    }

    // --- Stage gates -------------------------------------------------------

    #[test]
    fn a_scan_error_yields_only_scan_errors_and_no_tables() {
        let errors = evaluate("p % q", true).expect_err("the source must not scan");

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
        let errors = evaluate("p &", true).expect_err("the source must not parse");

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
        let errors = evaluate(&tautology_with_thirteen_variables(), false)
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
        let tables = evaluate("p & q\nq & p\nr\n", false).expect("the batch should evaluate");

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
        assert_eq!(evaluate("", true), Ok(vec![]));
        assert_eq!(evaluate("  \n\n", true), Ok(vec![]));
    }

    // --- Unchanged simplified text ----------------------------------------

    #[test]
    fn a_simplified_text_is_present_even_when_the_formula_does_not_change() {
        let table = one_table("p", true);

        assert_eq!(table.label, "p");
        assert_eq!(table.simplified, Some("p".to_string()));
    }

    // --- Orchestrator.simplify --------------------------------------------

    #[test]
    fn simplify_returns_the_normalized_formula() {
        assert_eq!(formulas("p -> q"), Ok(vec!["q ∨ ¬p".to_string()]));
    }

    #[test]
    fn simplify_collapses_a_thirteen_variable_tautology_without_a_size_error() {
        assert_eq!(
            formulas(&tautology_with_thirteen_variables()),
            Ok(vec!["V".to_string()])
        );
    }

    #[test]
    fn simplify_returns_one_formula_per_proposition_in_input_order() {
        assert_eq!(
            formulas("p & q\nr\n"),
            Ok(vec!["p ∧ q".to_string(), "r".to_string()])
        );
    }

    #[test]
    fn simplify_of_an_empty_source_returns_no_formulas() {
        assert_eq!(formulas(""), Ok(vec![]));
        assert_eq!(formulas("  \n\n"), Ok(vec![]));
    }

    #[test]
    fn simplify_gates_scan_errors_before_parsing() {
        let errors = formulas("p % q").expect_err("the source must not scan");

        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .all(|error| matches!(error, Error::ScanError { .. })),
            "mixed error kinds: {errors:?}"
        );
    }

    #[test]
    fn simplify_gates_parse_errors_before_simplifying() {
        let errors = formulas("p &").expect_err("the source must not parse");

        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .all(|error| matches!(error, Error::ParseError { .. })),
            "mixed error kinds: {errors:?}"
        );
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
