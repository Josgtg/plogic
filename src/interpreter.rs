//! The interpreter: evaluates a proposition under every truth assignment and
//! builds its truth table (`design/interpreter.md`).
//!
//! Pure and never constructed. It evaluates the expression it receives; the
//! Orchestrator chooses the original by default or the simplified one with
//! `--tabla-simplificada`, and the Interpreter does not know which one it is
//! evaluating (ADR-017). The only failure is the size limit: more than 4096
//! rows is refused before any row is enumerated (ADR-011).

use std::collections::HashMap;

use crate::error::Error;
use crate::expression::Expression;
use crate::tokens::TokenType;
use crate::truth_table::{Row, TruthTable};

/// The largest number of variables a table may have: 2^12 = 4096 rows.
const MAX_VARIABLES: usize = 12;

/// The interpreter stage. Never constructed.
pub struct Interpreter;

impl Interpreter {
    /// Evaluates `expression` under every truth assignment and returns its
    /// table. The table starts without display text; the Orchestrator adds it
    /// through [`TruthTable::with_display`] (ADR-020).
    ///
    /// Returns `Err` with one `EvalError` when the expression has more than 12
    /// distinct variables, before enumerating a single row.
    pub fn interpret(expression: &Expression) -> Result<TruthTable, Vec<Error>> {
        let variables = Self::variables(expression);
        if variables.len() > MAX_VARIABLES {
            return Err(vec![Error::EvalError {
                message: "resultado muy largo: más de 4096 filas".to_string(),
            }]);
        }
        let rows = Self::assignments(&variables)
            .into_iter()
            .map(|assignment| Row {
                values: variables
                    .iter()
                    .map(|name| assignment.get(name).copied().unwrap_or(false))
                    .collect(),
                result: Self::evaluate(expression, &assignment),
            })
            .collect();
        Ok(TruthTable {
            variables,
            rows,
            label: String::new(),
            simplified: None,
        })
    }

    /// The distinct variables of `expression`, in order of first appearance
    /// (an in-order walk of the AST).
    fn variables(expression: &Expression) -> Vec<String> {
        fn collect(expression: &Expression, variables: &mut Vec<String>) {
            match expression {
                Expression::Binary { left, right, .. } => {
                    collect(left, variables);
                    collect(right, variables);
                }
                Expression::Unary { expression, .. } => collect(expression, variables),
                Expression::Grouping { expression } => collect(expression, variables),
                Expression::Identifier { name } => {
                    if !variables.contains(name) {
                        variables.push(name.clone());
                    }
                }
                Expression::Literal { .. } => {}
            }
        }

        let mut variables = Vec::new();
        collect(expression, &mut variables);
        variables
    }

    /// Every truth assignment, in binary counting order with the last variable
    /// varying fastest: for `p`, `q` the rows are `F F`, `F V`, `V F`, `V V`.
    fn assignments(variables: &[String]) -> Vec<HashMap<String, bool>> {
        let count = 1_usize << variables.len();
        (0..count)
            .map(|row| {
                variables
                    .iter()
                    .enumerate()
                    .map(|(index, name)| {
                        let shift = variables.len() - 1 - index;
                        (name.clone(), (row >> shift) & 1 == 1)
                    })
                    .collect()
            })
            .collect()
    }

    /// The value of `expression` under `assignment`. The environment is built
    /// from the expression itself, so every identifier is present; the lookup
    /// still falls back to false instead of panicking (errors are values).
    fn evaluate(expression: &Expression, assignment: &HashMap<String, bool>) -> bool {
        match expression {
            Expression::Literal { value } => *value,
            Expression::Identifier { name } => assignment.get(name).copied().unwrap_or(false),
            Expression::Grouping { expression } => Self::evaluate(expression, assignment),
            Expression::Unary {
                operator,
                expression,
            } => {
                let value = Self::evaluate(expression, assignment);
                match operator.kind {
                    TokenType::Not => !value,
                    // Only `!` is unary in the grammar; treating anything else
                    // as transparent keeps the walk total.
                    _ => value,
                }
            }
            Expression::Binary {
                left,
                right,
                operator,
            } => {
                let left = Self::evaluate(left, assignment);
                let right = Self::evaluate(right, assignment);
                match operator.kind {
                    TokenType::And => left && right,
                    TokenType::Or => left || right,
                    TokenType::Implication => !left || right,
                    TokenType::Biconditional => left == right,
                    // The grammar has no other binary connective; false keeps
                    // the walk total without inventing a meaning.
                    _ => false,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Interpreter;
    use crate::error::Error;
    use crate::expression::Expression;

    fn parse(source: &str) -> Expression {
        let tokens = crate::scanner::scan(source.to_string()).expect("scan");
        let mut expressions = crate::parser::Parser::parse(tokens).expect("parse");
        assert_eq!(expressions.len(), 1, "source {source:?}");
        expressions.remove(0)
    }

    fn interpret(source: &str) -> crate::truth_table::TruthTable {
        Interpreter::interpret(&parse(source)).expect("the expression should evaluate")
    }

    fn result_column(source: &str) -> Vec<bool> {
        interpret(source)
            .rows
            .into_iter()
            .map(|row| row.result)
            .collect()
    }

    fn many_variables(count: usize) -> String {
        (1..=count)
            .map(|index| format!("p{index}"))
            .collect::<Vec<String>>()
            .join(" & ")
    }

    // --- Variable collection ----------------------------------------------

    #[test]
    fn variables_come_in_order_of_first_appearance() {
        assert_eq!(interpret("q & p").variables, ["q", "p"]);
    }

    #[test]
    fn duplicate_variables_are_counted_once() {
        let table = interpret("p & q & p");

        assert_eq!(table.variables, ["p", "q"]);
        assert_eq!(table.rows.len(), 4);
    }

    #[test]
    fn variables_inside_negations_and_groupings_are_collected() {
        assert_eq!(interpret("!(r) & p").variables, ["r", "p"]);
    }

    #[test]
    fn a_constant_has_no_variables() {
        assert!(interpret("V").variables.is_empty());
    }

    // --- Row order ---------------------------------------------------------

    #[test]
    fn rows_follow_binary_counting_with_the_last_variable_fastest() {
        let table = interpret("p -> q");

        assert_eq!(table.variables, ["p", "q"]);
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

    #[test]
    fn three_variable_rows_follow_binary_counting() {
        let table = interpret("p & q & r");

        assert_eq!(
            table
                .rows
                .iter()
                .map(|row| row.values.clone())
                .collect::<Vec<Vec<bool>>>(),
            [
                vec![false, false, false],
                vec![false, false, true],
                vec![false, true, false],
                vec![false, true, true],
                vec![true, false, false],
                vec![true, false, true],
                vec![true, true, false],
                vec![true, true, true],
            ]
        );
    }

    // --- Evaluation --------------------------------------------------------

    #[test]
    fn negation_inverts_the_value() {
        assert_eq!(result_column("!p"), [true, false]);
    }

    #[test]
    fn conjunction_is_true_only_when_both_sides_are_true() {
        assert_eq!(result_column("p & q"), [false, false, false, true]);
    }

    #[test]
    fn disjunction_is_false_only_when_both_sides_are_false() {
        assert_eq!(result_column("p | q"), [false, true, true, true]);
    }

    #[test]
    fn implication_is_false_only_when_true_implies_false() {
        assert_eq!(result_column("p -> q"), [true, true, false, true]);
    }

    #[test]
    fn biconditional_is_true_only_when_both_sides_agree() {
        assert_eq!(result_column("p <-> q"), [true, false, false, true]);
    }

    #[test]
    fn a_nested_formula_evaluates_bottom_up() {
        // !(p & q) | r: false only when p and q are true and r is false.
        assert_eq!(
            result_column("!(p & q) | r"),
            [true, true, true, true, true, true, false, true]
        );
    }

    // --- Smallest and largest tables --------------------------------------

    #[test]
    fn a_constant_true_gives_one_row_with_no_variables() {
        let table = interpret("V");

        assert!(table.variables.is_empty());
        assert_eq!(table.rows.len(), 1);
        assert!(table.rows[0].values.is_empty());
        assert!(table.rows[0].result);
    }

    #[test]
    fn a_tautology_still_keeps_its_variable_column() {
        let table = interpret("p | !p");

        assert_eq!(table.variables, ["p"]);
        assert_eq!(table.rows.len(), 2);
        assert!(table.rows.iter().all(|row| row.result));
    }

    #[test]
    fn twelve_variables_produce_four_thousand_and_ninety_six_rows() {
        let table = interpret(&many_variables(12));

        assert_eq!(table.variables.len(), 12);
        assert_eq!(table.rows.len(), 4096);
        assert!(table.rows.iter().all(|row| row.values.len() == 12));
    }

    // --- Size limit --------------------------------------------------------

    #[test]
    fn thirteen_variables_are_refused_with_the_size_limit_error() {
        let errors = Interpreter::interpret(&parse(&many_variables(13)))
            .expect_err("13 variables must exceed the limit");

        assert_eq!(
            errors,
            [Error::EvalError {
                message: "resultado muy largo: más de 4096 filas".to_string(),
            }]
        );
    }

    #[test]
    fn sixty_four_variables_are_refused_without_overflowing() {
        let errors = Interpreter::interpret(&parse(&many_variables(64)))
            .expect_err("64 variables must exceed the limit");

        assert_eq!(
            errors,
            [Error::EvalError {
                message: "resultado muy largo: más de 4096 filas".to_string(),
            }]
        );
    }

    // --- Display fields ----------------------------------------------------

    #[test]
    fn the_interpreter_leaves_the_display_fields_empty() {
        let table = interpret("p & q");

        assert_eq!(table.label, "");
        assert_eq!(table.simplified, None);
    }

    // --- Independence from the simplifier ----------------------------------

    #[test]
    fn the_interpreter_does_not_simplify_the_expression() {
        let table = interpret("p & (q | r)");

        assert_eq!(table.variables, ["p", "q", "r"]);
        assert_eq!(table.rows.len(), 8);
    }
}
