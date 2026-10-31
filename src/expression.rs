//! The abstract syntax tree (`design/parser.md`, `design/diagrams/expression_class.puml`).
//!
//! `Expression` is pure data: a closed hierarchy with no methods, traversed by
//! pattern matching instead of a visitor (ADR-001). The parser creates the
//! nodes; [`ASTPrinter`] renders them in parenthesized prefix notation for
//! verification and [`InfixPrinter`] renders the normalized infix form the user
//! sees (ADR-009, ADR-015). Both printers are static and stateless.

use crate::tokens::{Token, TokenType};

/// The nodes of the abstract syntax tree.
///
/// Equality is structural but ignores where the operator was written: two
/// trees with the same shape and the same operators are equal even if their
/// operator tokens come from different positions in the source (ADR-040).
/// The Simplifier relies on that when it compares operand lists.
#[derive(Debug, Clone)]
pub enum Expression {
    Binary {
        left: Box<Expression>,
        right: Box<Expression>,
        operator: Token,
    },
    Unary {
        operator: Token,
        expression: Box<Expression>,
    },
    Grouping {
        expression: Box<Expression>,
    },
    Literal {
        value: bool,
    },
    Identifier {
        name: String,
    },
}

impl PartialEq for Expression {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Expression::Binary {
                    left,
                    right,
                    operator,
                },
                Expression::Binary {
                    left: other_left,
                    right: other_right,
                    operator: other_operator,
                },
            ) => left == other_left && right == other_right && operator.kind == other_operator.kind,
            (
                Expression::Unary {
                    operator,
                    expression,
                },
                Expression::Unary {
                    operator: other_operator,
                    expression: other_expression,
                },
            ) => operator.kind == other_operator.kind && expression == other_expression,
            (
                Expression::Grouping { expression },
                Expression::Grouping {
                    expression: other_expression,
                },
            ) => expression == other_expression,
            (Expression::Literal { value }, Expression::Literal { value: other_value }) => {
                value == other_value
            }
            (Expression::Identifier { name }, Expression::Identifier { name: other_name }) => {
                name == other_name
            }
            _ => false,
        }
    }
}

impl Eq for Expression {}

/// Renders an expression in parenthesized prefix (Polish) notation, used to
/// verify the tree. Its parentheses never depend on precedence; a `Grouping`
/// prints as `(group …)`, the way the book's printer does, so redundant
/// parentheses in the source stay visible.
pub struct ASTPrinter;

impl ASTPrinter {
    /// The prefix rendering of `expression`, e.g. `(∧ (¬ p) q)`.
    pub fn print(expression: &Expression) -> String {
        match expression {
            Expression::Binary {
                left,
                right,
                operator,
            } => Self::print_binary(left, operator, right),
            Expression::Unary {
                operator,
                expression,
            } => Self::print_unary(operator, expression),
            Expression::Grouping { expression } => Self::print_grouping(expression),
            Expression::Literal { value } => Self::print_literal(*value),
            Expression::Identifier { name } => Self::print_identifier(name),
        }
    }

    fn print_binary(left: &Expression, operator: &Token, right: &Expression) -> String {
        format!(
            "({} {} {})",
            operator.kind,
            Self::print(left),
            Self::print(right)
        )
    }

    fn print_unary(operator: &Token, expression: &Expression) -> String {
        format!("({} {})", operator.kind, Self::print(expression))
    }

    fn print_grouping(expression: &Expression) -> String {
        format!("(group {})", Self::print(expression))
    }

    fn print_literal(value: bool) -> String {
        TokenType::Literal(value).to_string()
    }

    fn print_identifier(name: &str) -> String {
        name.to_string()
    }
}

/// Renders an expression in the normalized infix form the user sees: `¬`, `∧`,
/// `∨`, `→`, `↔`, literals `V` / `F`, operands in the order they were written
/// and only the parentheses precedence or associativity requires (ADR-009).
///
/// A `Grouping` is transparent: the parentheses it records are printed only
/// when the inner expression would otherwise be misread. `(p | q) & r` keeps
/// them; `(p)` and `!p & q` do not gain any.
pub struct InfixPrinter;

impl InfixPrinter {
    /// The infix rendering of `expression`, e.g. `¬p ∧ q`.
    pub fn print(expression: &Expression) -> String {
        Self::print_with_minimum(expression, 0)
    }

    /// Prints `expression`, adding parentheses around it when its precedence is
    /// below `minimum`. The caller raises the minimum for the side of an
    /// operator that associativity does not let it join.
    fn print_with_minimum(expression: &Expression, minimum: u8) -> String {
        match expression {
            Expression::Binary {
                left,
                right,
                operator,
            } => Self::print_binary(left, operator, right, minimum),
            Expression::Unary {
                operator,
                expression,
            } => Self::print_unary(operator, expression),
            Expression::Grouping { expression } => Self::print_grouping(expression, minimum),
            Expression::Literal { value } => Self::print_literal(*value),
            Expression::Identifier { name } => Self::print_identifier(name),
        }
    }

    fn print_binary(
        left: &Expression,
        operator: &Token,
        right: &Expression,
        minimum: u8,
    ) -> String {
        let precedence = Self::precedence_of_binary(&operator.kind);
        // Implication is the only right-associative level, so its own
        // precedence may extend to the right but not to the left. Conjunction
        // and disjunction also let it extend to the right: they are
        // associative, so `p ∧ (q ∧ r)` reads unambiguously as `p ∧ q ∧ r`
        // and the parentheses are dropped. The biconditional keeps them,
        // because the design prints `p ↔ (q ↔ r)` explicitly.
        let left_minimum = if operator.kind == TokenType::Implication {
            precedence + 1
        } else {
            precedence
        };
        let right_minimum = if matches!(
            operator.kind,
            TokenType::Implication | TokenType::And | TokenType::Or
        ) {
            precedence
        } else {
            precedence + 1
        };

        let printed = format!(
            "{} {} {}",
            Self::print_with_minimum(left, left_minimum),
            operator.kind,
            Self::print_with_minimum(right, right_minimum)
        );

        Self::parenthesize(printed, precedence, minimum)
    }

    fn print_unary(operator: &Token, expression: &Expression) -> String {
        format!(
            "{}{}",
            operator.kind,
            Self::print_with_minimum(expression, 5)
        )
    }

    fn print_grouping(expression: &Expression, minimum: u8) -> String {
        Self::print_with_minimum(expression, minimum)
    }

    fn print_literal(value: bool) -> String {
        TokenType::Literal(value).to_string()
    }

    fn print_identifier(name: &str) -> String {
        name.to_string()
    }

    fn precedence_of_binary(kind: &TokenType) -> u8 {
        match kind {
            TokenType::Biconditional => 1,
            TokenType::Implication => 2,
            TokenType::Or => 3,
            TokenType::And => 4,
            _ => 6,
        }
    }

    fn parenthesize(printed: String, precedence: u8, minimum: u8) -> String {
        if precedence < minimum {
            format!("({printed})")
        } else {
            printed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ASTPrinter, Expression, InfixPrinter};
    use crate::tokens::{Token, TokenType};

    fn token(kind: TokenType) -> Token {
        Token {
            kind,
            line: 1,
            col: 1,
        }
    }

    fn token_at(kind: TokenType, line: usize, col: usize) -> Token {
        Token { kind, line, col }
    }

    fn id(name: &str) -> Expression {
        Expression::Identifier {
            name: name.to_string(),
        }
    }

    fn literal(value: bool) -> Expression {
        Expression::Literal { value }
    }

    fn unary(kind: TokenType, expression: Expression) -> Expression {
        Expression::Unary {
            operator: token(kind),
            expression: Box::new(expression),
        }
    }

    fn binary(left: Expression, kind: TokenType, right: Expression) -> Expression {
        Expression::Binary {
            left: Box::new(left),
            right: Box::new(right),
            operator: token(kind),
        }
    }

    fn grouping(expression: Expression) -> Expression {
        Expression::Grouping {
            expression: Box::new(expression),
        }
    }

    fn ast(expression: &Expression) -> String {
        ASTPrinter::print(expression)
    }

    fn infix(expression: &Expression) -> String {
        InfixPrinter::print(expression)
    }

    fn product_example() -> Expression {
        binary(
            binary(
                binary(
                    id("p"),
                    TokenType::Or,
                    binary(id("q"), TokenType::And, id("s")),
                ),
                TokenType::Implication,
                id("t"),
            ),
            TokenType::Biconditional,
            binary(id("q"), TokenType::Or, literal(false)),
        )
    }

    #[test]
    fn equal_trees_ignore_operator_positions() {
        let first = Expression::Binary {
            left: Box::new(Expression::Unary {
                operator: token_at(TokenType::Not, 1, 1),
                expression: Box::new(id("p")),
            }),
            right: Box::new(id("q")),
            operator: token_at(TokenType::And, 1, 4),
        };
        let second = Expression::Binary {
            left: Box::new(Expression::Unary {
                operator: token_at(TokenType::Not, 7, 2),
                expression: Box::new(id("p")),
            }),
            right: Box::new(id("q")),
            operator: token_at(TokenType::And, 3, 9),
        };

        assert_eq!(first, second);
        assert_eq!(second, first);
    }

    #[test]
    fn different_operator_kinds_are_not_equal() {
        assert_ne!(
            binary(id("p"), TokenType::And, id("q")),
            binary(id("p"), TokenType::Or, id("q"))
        );
    }

    #[test]
    fn different_operands_are_not_equal() {
        assert_ne!(
            binary(id("p"), TokenType::And, id("q")),
            binary(id("p"), TokenType::And, id("r"))
        );
    }

    #[test]
    fn different_nesting_is_not_equal() {
        let left_nested = binary(
            binary(id("p"), TokenType::And, id("q")),
            TokenType::And,
            id("r"),
        );
        let right_nested = binary(
            id("p"),
            TokenType::And,
            binary(id("q"), TokenType::And, id("r")),
        );

        assert_ne!(left_nested, right_nested);
    }

    #[test]
    fn a_grouping_is_not_its_bare_child() {
        assert_ne!(grouping(id("p")), id("p"));
    }

    #[test]
    fn equality_is_reflexive_and_symmetric() {
        let tree = product_example();
        let copy = product_example();

        assert_eq!(tree, tree);
        assert_eq!(tree, copy);
        assert_eq!(copy, tree);
        assert_ne!(tree, binary(id("p"), TokenType::Or, id("q")));
    }

    #[test]
    fn ast_printer_prints_literals_and_identifiers() {
        assert_eq!(ast(&literal(true)), "V");
        assert_eq!(ast(&literal(false)), "F");
        assert_eq!(ast(&id("p")), "p");
    }

    #[test]
    fn ast_printer_prints_a_negated_conjunction() {
        let expression = binary(unary(TokenType::Not, id("p")), TokenType::And, id("q"));

        assert_eq!(ast(&expression), "(∧ (¬ p) q)");
    }

    #[test]
    fn ast_printer_keeps_redundant_groupings() {
        assert_eq!(ast(&grouping(id("p"))), "(group p)");
        assert_eq!(
            ast(&grouping(binary(id("p"), TokenType::And, id("q")))),
            "(group (∧ p q))"
        );
    }

    #[test]
    fn ast_printer_prints_the_product_example() {
        assert_eq!(ast(&product_example()), "(↔ (→ (∨ p (∧ q s)) t) (∨ q F))");
    }

    #[test]
    fn ast_printer_uses_single_spaces_between_tokens() {
        for expression in [
            literal(true),
            id("p"),
            unary(TokenType::Not, id("p")),
            grouping(id("p")),
            product_example(),
        ] {
            let printed = ast(&expression);
            assert!(!printed.contains("  "), "double space in {printed:?}");
            assert!(!printed.starts_with(' '), "leading space in {printed:?}");
            assert!(!printed.ends_with(' '), "trailing space in {printed:?}");
        }
    }

    #[test]
    fn infix_printer_prints_a_negated_conjunction() {
        let expression = binary(unary(TokenType::Not, id("p")), TokenType::And, id("q"));

        assert_eq!(infix(&expression), "¬p ∧ q");
    }

    #[test]
    fn infix_printer_drops_redundant_groupings() {
        assert_eq!(infix(&grouping(id("p"))), "p");
        assert_eq!(
            infix(&grouping(binary(id("p"), TokenType::And, id("q")))),
            "p ∧ q"
        );
    }

    #[test]
    fn infix_printer_keeps_groupings_that_precedence_requires() {
        let expression = binary(
            grouping(binary(id("p"), TokenType::Or, id("q"))),
            TokenType::And,
            id("r"),
        );

        assert_eq!(infix(&expression), "(p ∨ q) ∧ r");
    }

    #[test]
    fn infix_printer_drops_groupings_that_associativity_makes_redundant() {
        let expression = binary(
            id("p"),
            TokenType::And,
            grouping(binary(id("q"), TokenType::And, id("r"))),
        );

        assert_eq!(infix(&expression), "p ∧ q ∧ r");
    }

    #[test]
    fn infix_printer_respects_the_precedence_ladder() {
        assert_eq!(infix(&product_example()), "p ∨ q ∧ s → t ↔ q ∨ F");
    }

    #[test]
    fn infix_printer_prints_implication_as_right_associative() {
        let expression = binary(
            id("p"),
            TokenType::Implication,
            binary(id("q"), TokenType::Implication, id("r")),
        );

        assert_eq!(infix(&expression), "p → q → r");
    }

    #[test]
    fn infix_printer_parenthesizes_a_left_nested_implication() {
        let expression = binary(
            binary(id("p"), TokenType::Implication, id("q")),
            TokenType::Implication,
            id("r"),
        );

        assert_eq!(infix(&expression), "(p → q) → r");
    }

    #[test]
    fn infix_printer_prints_biconditional_as_left_associative() {
        let expression = binary(
            binary(id("p"), TokenType::Biconditional, id("q")),
            TokenType::Biconditional,
            id("r"),
        );

        assert_eq!(infix(&expression), "p ↔ q ↔ r");
    }

    #[test]
    fn infix_printer_parenthesizes_a_right_nested_biconditional() {
        let expression = binary(
            id("p"),
            TokenType::Biconditional,
            binary(id("q"), TokenType::Biconditional, id("r")),
        );

        assert_eq!(infix(&expression), "p ↔ (q ↔ r)");
    }

    #[test]
    fn infix_printer_prints_left_nested_conjunctions_and_disjunctions() {
        let conjunction = binary(
            binary(id("p"), TokenType::And, id("q")),
            TokenType::And,
            id("r"),
        );
        let disjunction = binary(
            binary(id("p"), TokenType::Or, id("q")),
            TokenType::Or,
            id("r"),
        );

        assert_eq!(infix(&conjunction), "p ∧ q ∧ r");
        assert_eq!(infix(&disjunction), "p ∨ q ∨ r");
    }

    #[test]
    fn infix_printer_parenthesizes_a_negated_binary() {
        let expression = unary(
            TokenType::Not,
            grouping(binary(id("p"), TokenType::And, id("q"))),
        );

        assert_eq!(infix(&expression), "¬(p ∧ q)");
    }

    #[test]
    fn infix_printer_prints_double_negation() {
        let expression = unary(TokenType::Not, unary(TokenType::Not, id("p")));

        assert_eq!(infix(&expression), "¬¬p");
    }

    #[test]
    fn infix_printer_prints_a_negated_literal() {
        assert_eq!(infix(&unary(TokenType::Not, literal(true))), "¬V");
        assert_eq!(infix(&unary(TokenType::Not, literal(false))), "¬F");
    }

    #[test]
    fn infix_printer_prints_a_negation_under_a_binary_without_stray_parentheses() {
        let expression = binary(id("p"), TokenType::And, unary(TokenType::Not, id("q")));

        assert_eq!(infix(&expression), "p ∧ ¬q");
    }
}
