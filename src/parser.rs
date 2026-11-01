//! The parser stage: tokens to syntax trees (`design/parser.md`).
//!
//! A recursive-descent parser, one function per precedence level, following
//! Crafting Interpreters ch. 6 (ADR-001). [`Parser::parse`] is the book's
//! statement loop: it parses one proposition per line, recovers from a bad
//! line by synchronizing to the next newline, and returns every error it
//! collected, or the whole list of expressions (ADR-006, ADR-025).
//!
//! The immutable [`ParseState`] owns the token list and is threaded through
//! every rule, which returns it anew. A rule failure carries the state where
//! parsing stopped, so `parse` can synchronize from exactly there (ADR-025).

use crate::error::Error;
use crate::expression::Expression;
use crate::tokens::{Token, TokenType};

/// A rule's result: the node and the state after it, or the error and the
/// state at the token that caused it (ADR-025).
type RuleResult = Result<(Expression, ParseState), (Error, ParseState)>;

/// The position a [`Token`] is read from when the list runs out.
///
/// The scanner always ends its output with `Eof`, so this only exists so that
/// a hand-built token list cannot make a rule read past its end (ADR-041).
const EOF_TOKEN: Token = Token {
    kind: TokenType::Eof,
    line: 0,
    col: 0,
};

/// The parser's cursor: the tokens and the index of the next one to read.
///
/// It owns the list, like the scanner's state owns the source (ADR-038), so no
/// helper carries a lifetime (ADR-041).
#[derive(Debug, Clone)]
struct ParseState {
    tokens: Vec<Token>,
    current: usize,
}

/// The parser stage.
///
/// Never constructed: [`Parser::parse`] is the static entry point (ADR-001).
pub struct Parser;

impl Parser {
    /// Parses a token list into one expression per proposition.
    ///
    /// Blank lines are empty propositions and are skipped. A failing
    /// proposition records its error and the parser synchronizes to the next
    /// line, so as many errors as possible are reported; the stage still
    /// returns `Err` with all of them and discards the partial expressions
    /// (`design/error_handling.md`).
    pub fn parse(tokens: Vec<Token>) -> Result<Vec<Expression>, Vec<Error>> {
        let mut state = ParseState { tokens, current: 0 };
        let mut expressions = Vec::new();
        let mut errors = Vec::new();

        while !is_at_end(&state) {
            let (newline, next) = match_token(state, is_newline);
            if newline.is_some() {
                state = next;
                continue;
            }

            match proposition(next) {
                Ok((expression, next)) => {
                    expressions.push(expression);
                    state = next;
                }
                Err((error, failing)) => {
                    errors.push(error);
                    state = synchronize(failing);
                }
            }
        }

        if errors.is_empty() {
            Ok(expressions)
        } else {
            Err(errors)
        }
    }
}

/// `proposition -> expression ( Newline | Eof )`.
///
/// The terminating newline is consumed; `Eof` is not, because it is read many
/// times at the end of the script.
fn proposition(state: ParseState) -> RuleResult {
    let (expression, state) = expression(state)?;

    if is_at_end(&state) {
        return Ok((expression, state));
    }

    let (newline, next) = match_token(state, is_newline);
    if newline.is_some() {
        return Ok((expression, next));
    }

    let token = peek(&next);
    Err((
        operator_error(token, "se esperaba el fin de la proposición"),
        next,
    ))
}

/// `expression -> biconditional`.
fn expression(state: ParseState) -> RuleResult {
    biconditional(state)
}

/// `biconditional -> implication ( Biconditional implication )*`.
///
/// Left-associative: `p <-> q <-> r` is `(p <-> q) <-> r`.
fn biconditional(state: ParseState) -> RuleResult {
    let (mut left, mut state) = implication(state)?;

    loop {
        let (operator, next) = match_token(state, is_biconditional);
        let Some(operator) = operator else {
            return Ok((left, next));
        };

        let (right, next) = implication(next)?;
        state = next;
        left = binary(left, operator, right);
    }
}

/// `implication -> disjunction ( Implication implication )?`.
///
/// Right-associative: the rule recurses into itself, so `p -> q -> r` means
/// `p -> (q -> r)`.
fn implication(state: ParseState) -> RuleResult {
    let (left, state) = disjunction(state)?;

    let (operator, next) = match_token(state, is_implication);
    let Some(operator) = operator else {
        return Ok((left, next));
    };

    let (right, next) = implication(next)?;
    Ok((binary(left, operator, right), next))
}

/// `disjunction -> conjunction ( Or conjunction )*`.
fn disjunction(state: ParseState) -> RuleResult {
    let (mut left, mut state) = conjunction(state)?;

    loop {
        let (operator, next) = match_token(state, is_or);
        let Some(operator) = operator else {
            return Ok((left, next));
        };

        let (right, next) = conjunction(next)?;
        state = next;
        left = binary(left, operator, right);
    }
}

/// `conjunction -> unary ( And unary )*`.
fn conjunction(state: ParseState) -> RuleResult {
    let (mut left, mut state) = unary(state)?;

    loop {
        let (operator, next) = match_token(state, is_and);
        let Some(operator) = operator else {
            return Ok((left, next));
        };

        let (right, next) = unary(next)?;
        state = next;
        left = binary(left, operator, right);
    }
}

/// `unary -> Not unary | primary`.
fn unary(state: ParseState) -> RuleResult {
    let (operator, next) = match_token(state, is_not);
    let Some(operator) = operator else {
        return primary(next);
    };

    let (expression, next) = unary(next)?;
    Ok((
        Expression::Unary {
            operator,
            expression: Box::new(expression),
        },
        next,
    ))
}

/// `primary -> Literal | Identifier | "(" expression ")"`.
fn primary(state: ParseState) -> RuleResult {
    let token = peek(&state).clone();

    match token.kind {
        TokenType::Literal(value) => {
            let (_, state) = advance(state);
            Ok((Expression::Literal { value }, state))
        }
        TokenType::Identifier(name) => {
            let (_, state) = advance(state);
            Ok((Expression::Identifier { name }, state))
        }
        TokenType::LeftParen => {
            let (_, state) = advance(state);
            let (expression, state) = expression(state)?;
            let (_, state) = consume(
                state,
                is_right_paren,
                "se esperaba ')' después de la expresión",
            )?;
            Ok((
                Expression::Grouping {
                    expression: Box::new(expression),
                },
                state,
            ))
        }
        _ => Err((error(&token, "se esperaba una expresión"), state)),
    }
}

/// Builds a binary node.
fn binary(left: Expression, operator: Token, right: Expression) -> Expression {
    Expression::Binary {
        left: Box::new(left),
        right: Box::new(right),
        operator,
    }
}

/// Reads the next token if it satisfies `predicate`, advancing past it.
///
/// The predicates exist because `TokenType` carries payloads, so a value
/// comparison would have to invent a literal or an identifier to test against
/// (ADR-041).
fn match_token(
    state: ParseState,
    predicate: impl Fn(&TokenType) -> bool,
) -> (Option<Token>, ParseState) {
    if !check(&state, &predicate) {
        return (None, state);
    }
    let (token, state) = advance(state);
    (Some(token), state)
}

/// Reads a required token, or reports `message` at the token that should have
/// been it.
fn consume(
    state: ParseState,
    predicate: impl Fn(&TokenType) -> bool,
    message: &str,
) -> Result<(Token, ParseState), (Error, ParseState)> {
    let (token, state) = match_token(state, predicate);
    match token {
        Some(token) => Ok((token, state)),
        None => Err((operator_error(peek(&state), message), state)),
    }
}

/// Whether the token under the cursor satisfies `predicate`.
fn check(state: &ParseState, predicate: impl Fn(&TokenType) -> bool) -> bool {
    state
        .tokens
        .get(state.current)
        .is_some_and(|token| predicate(&token.kind))
}

/// Whether the cursor is at `Eof` or past the end of the token list.
fn is_at_end(state: &ParseState) -> bool {
    state
        .tokens
        .get(state.current)
        .is_none_or(|token| matches!(token.kind, TokenType::Eof))
}

/// The token under the cursor, or [`EOF_TOKEN`] when the list runs out.
fn peek(state: &ParseState) -> &Token {
    state.tokens.get(state.current).unwrap_or(&EOF_TOKEN)
}

/// The token under the cursor, and the state just past it.
fn advance(state: ParseState) -> (Token, ParseState) {
    let token = peek(&state).clone();
    let current = if state.current < state.tokens.len() {
        state.current + 1
    } else {
        state.current
    };
    (token, ParseState { current, ..state })
}

/// The `ParseError` for a token found where an operator belongs.
///
/// `v` is a common mistyping for the disjunction, but it scans as a variable
/// name, so the parser sees it as a leftover token. It gets a message that
/// says it is not an operator and lists the accepted spellings (ADR-050);
/// every other token keeps the caller's message.
fn operator_error(token: &Token, message: &str) -> Error {
    if matches!(&token.kind, TokenType::Identifier(name) if name == "v") {
        error(
            token,
            "'v' no es un operador; usa '|', '∨' u 'o' para la disyunción",
        )
    } else {
        error(token, message)
    }
}

/// The `ParseError` for a token and the message saying what was expected.
fn error(token: &Token, message: &str) -> Error {
    Error::ParseError {
        line: token.line,
        col: token.col,
        message: message.to_string(),
    }
}

/// Skips tokens until just after the next `Newline`, or until the input ends.
///
/// It starts from the state a failing rule returned, which points at the token
/// that caused the error, so recovery resumes from exactly there (ADR-025).
fn synchronize(state: ParseState) -> ParseState {
    let mut state = state;

    while !is_at_end(&state) {
        let (newline, next) = match_token(state, is_newline);
        if newline.is_some() {
            return next;
        }
        let (_, next) = advance(next);
        state = next;
    }

    state
}

fn is_newline(kind: &TokenType) -> bool {
    matches!(kind, TokenType::Newline)
}

fn is_not(kind: &TokenType) -> bool {
    matches!(kind, TokenType::Not)
}

fn is_and(kind: &TokenType) -> bool {
    matches!(kind, TokenType::And)
}

fn is_or(kind: &TokenType) -> bool {
    matches!(kind, TokenType::Or)
}

fn is_implication(kind: &TokenType) -> bool {
    matches!(kind, TokenType::Implication)
}

fn is_biconditional(kind: &TokenType) -> bool {
    matches!(kind, TokenType::Biconditional)
}

fn is_right_paren(kind: &TokenType) -> bool {
    matches!(kind, TokenType::RightParen)
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use crate::error::Error;
    use crate::expression::{ASTPrinter, Expression, InfixPrinter};

    /// Scans first, so the tests are about parsing and assume a clean token
    /// stream; the scanner has its own tests.
    fn parse(source: &str) -> Result<Vec<Expression>, Vec<Error>> {
        let tokens = crate::scanner::scan(source.to_string()).expect("the source should scan");
        Parser::parse(tokens)
    }

    fn parse_one(source: &str) -> Expression {
        let mut expressions = parse(source).expect("the source should parse");
        assert_eq!(expressions.len(), 1, "source {source:?}");
        expressions.remove(0)
    }

    fn ast(source: &str) -> String {
        ASTPrinter::print(&parse_one(source))
    }

    fn infix(source: &str) -> String {
        InfixPrinter::print(&parse_one(source))
    }

    fn print_all(expressions: &[Expression]) -> Vec<String> {
        expressions.iter().map(ASTPrinter::print).collect()
    }

    fn parse_errors(source: &str) -> Vec<Error> {
        parse(source).expect_err("the source should not parse")
    }

    fn rendered_errors(source: &str) -> Vec<String> {
        parse_errors(source)
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    // --- Precedence: one case per adjacent level pair -----------------------

    #[test]
    fn conjunction_binds_tighter_than_disjunction() {
        assert_eq!(ast("p & q | r"), "(∨ (∧ p q) r)");
    }

    #[test]
    fn negation_binds_tighter_than_conjunction() {
        assert_eq!(ast("!p & q"), "(∧ (¬ p) q)");
    }

    #[test]
    fn disjunction_binds_tighter_than_implication() {
        assert_eq!(ast("p | q -> r"), "(→ (∨ p q) r)");
    }

    #[test]
    fn implication_binds_tighter_than_biconditional() {
        assert_eq!(ast("p -> q <-> r"), "(↔ (→ p q) r)");
    }

    // --- Associativity ------------------------------------------------------

    #[test]
    fn implication_is_right_associative() {
        assert_eq!(ast("p -> q -> r"), "(→ p (→ q r))");
    }

    #[test]
    fn biconditional_is_left_associative() {
        assert_eq!(ast("p <-> q <-> r"), "(↔ (↔ p q) r)");
    }

    #[test]
    fn conjunction_is_left_associative() {
        assert_eq!(ast("p & q & r"), "(∧ (∧ p q) r)");
    }

    #[test]
    fn disjunction_is_left_associative() {
        assert_eq!(ast("p | q | r"), "(∨ (∨ p q) r)");
    }

    // --- The product example ------------------------------------------------

    #[test]
    fn the_product_example_parses_with_the_documented_shape() {
        assert_eq!(
            ast("p | q & s -> t <-> q | 0"),
            "(↔ (→ (∨ p (∧ q s)) t) (∨ q F))"
        );
        assert_eq!(infix("p | q & s -> t <-> q | 0"), "p ∨ q ∧ s → t ↔ q ∨ F");
    }

    // --- Grouping -----------------------------------------------------------

    #[test]
    fn parentheses_override_precedence() {
        assert_eq!(ast("(p | q) & r"), "(∧ (group (∨ p q)) r)");
    }

    #[test]
    fn a_redundant_grouping_stays_in_the_ast() {
        assert_eq!(ast("(p)"), "(group p)");
    }

    #[test]
    fn nested_redundant_groupings_stay_in_the_ast() {
        assert_eq!(ast("((p))"), "(group (group p))");
        assert_eq!(infix("((p))"), "p");
    }

    #[test]
    fn infix_printer_drops_a_redundant_grouping() {
        assert_eq!(infix("(p)"), "p");
    }

    // --- Printers after parsing --------------------------------------------

    #[test]
    fn printers_render_a_negated_conjunction() {
        assert_eq!(ast("!p & q"), "(∧ (¬ p) q)");
        assert_eq!(infix("!p & q"), "¬p ∧ q");
    }

    #[test]
    fn infix_printer_keeps_the_parentheses_precedence_requires() {
        assert_eq!(infix("(p | q) & r"), "(p ∨ q) ∧ r");
    }

    #[test]
    fn infix_printer_parenthesizes_a_left_nested_implication() {
        assert_eq!(infix("(p -> q) -> r"), "(p → q) → r");
    }

    #[test]
    fn infix_printer_drops_parentheses_around_a_right_nested_implication() {
        assert_eq!(infix("p -> (q -> r)"), "p → q → r");
    }

    #[test]
    fn infix_printer_drops_parentheses_around_a_right_nested_conjunction() {
        assert_eq!(infix("p & (q & r)"), "p ∧ q ∧ r");
    }

    // --- Script loop --------------------------------------------------------

    #[test]
    fn script_loop_parses_one_proposition_per_line_in_order() {
        let expressions = parse("p\nq | r\n!s").expect("three propositions should parse");

        assert_eq!(print_all(&expressions), ["p", "(∨ q r)", "(¬ s)"]);
    }

    #[test]
    fn blank_lines_are_skipped() {
        let expressions = parse("\np\n\nq\n\n").expect("two propositions should parse");

        assert_eq!(print_all(&expressions), ["p", "q"]);
    }

    #[test]
    fn the_last_proposition_needs_no_trailing_newline() {
        let expressions = parse("p\nq").expect("two propositions should parse");

        assert_eq!(print_all(&expressions), ["p", "q"]);
    }

    #[test]
    fn a_single_proposition_needs_no_trailing_newline() {
        assert_eq!(parse_one("p"), parse_one("p\n"));
    }

    #[test]
    fn an_empty_source_has_no_propositions() {
        assert!(parse("").expect("an empty source should parse").is_empty());
    }

    #[test]
    fn a_source_of_only_blank_lines_has_no_propositions() {
        assert!(
            parse("\n\n\n")
                .expect("a blank source should parse")
                .is_empty()
        );
    }

    // --- Error recovery -----------------------------------------------------

    #[test]
    fn recovery_reports_each_bad_line_and_discards_the_expressions() {
        let errors = parse_errors("p\np &\nq &\nr");

        assert_eq!(
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>(),
            [
                "línea 2, columna 4: se esperaba una expresión",
                "línea 3, columna 4: se esperaba una expresión",
            ]
        );
        assert!(
            errors
                .iter()
                .all(|error| matches!(error, Error::ParseError { .. })),
            "every error should be a ParseError: {errors:?}"
        );
    }

    #[test]
    fn several_bad_lines_report_every_error_in_source_order() {
        let errors = parse_errors("p &\nq |\nr s");

        assert_eq!(
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>(),
            [
                "línea 1, columna 4: se esperaba una expresión",
                "línea 2, columna 4: se esperaba una expresión",
                "línea 3, columna 3: se esperaba el fin de la proposición",
            ]
        );
        assert!(
            errors
                .iter()
                .all(|error| matches!(error, Error::ParseError { .. })),
            "every error should be a ParseError: {errors:?}"
        );
    }

    #[test]
    fn one_good_line_and_one_bad_line_return_err_without_partial_expressions() {
        let errors = parse_errors("p\nq &");

        assert_eq!(
            errors,
            [Error::ParseError {
                line: 2,
                col: 4,
                message: "se esperaba una expresión".to_string(),
            }]
        );
    }

    // --- Error positions ----------------------------------------------------

    #[test]
    fn a_dangling_conjunction_reports_the_expected_expression_at_the_line_end() {
        assert_eq!(
            rendered_errors("p &"),
            ["línea 1, columna 4: se esperaba una expresión"]
        );
    }

    #[test]
    fn an_unclosed_group_reports_the_missing_parenthesis() {
        assert_eq!(
            rendered_errors("(p | q"),
            ["línea 1, columna 7: se esperaba ')' después de la expresión"]
        );
    }

    #[test]
    fn a_missing_line_break_reports_that_the_proposition_did_not_end() {
        assert_eq!(
            rendered_errors("p q"),
            ["línea 1, columna 3: se esperaba el fin de la proposición"]
        );
    }

    #[test]
    fn an_empty_group_reports_the_expected_expression_at_the_closing_parenthesis() {
        assert_eq!(
            rendered_errors("()"),
            ["línea 1, columna 2: se esperaba una expresión"]
        );
    }

    #[test]
    fn a_lone_negation_reports_the_expected_expression_at_the_end() {
        assert_eq!(
            rendered_errors("!"),
            ["línea 1, columna 2: se esperaba una expresión"]
        );
    }

    // --- The `v` disjunction typo (ADR-050) --------------------------------

    #[test]
    fn a_v_where_an_operator_belongs_explains_the_disjunction_spellings() {
        assert_eq!(
            rendered_errors("p v q"),
            ["línea 1, columna 3: 'v' no es un operador; usa '|', '∨' u 'o' para la disyunción"]
        );
    }

    #[test]
    fn a_trailing_v_after_a_proposition_explains_the_disjunction_spellings() {
        assert_eq!(
            rendered_errors("p v"),
            ["línea 1, columna 3: 'v' no es un operador; usa '|', '∨' u 'o' para la disyunción"]
        );
    }

    #[test]
    fn a_v_before_a_required_parenthesis_explains_the_disjunction_spellings() {
        assert_eq!(
            rendered_errors("(p v q)"),
            ["línea 1, columna 4: 'v' no es un operador; usa '|', '∨' u 'o' para la disyunción"]
        );
    }

    #[test]
    fn a_v_midway_through_a_longer_proposition_explains_the_disjunction_spellings() {
        assert_eq!(
            rendered_errors("p v q & r"),
            ["línea 1, columna 3: 'v' no es un operador; usa '|', '∨' u 'o' para la disyunción"]
        );
    }

    #[test]
    fn another_identifier_after_a_proposition_keeps_the_generic_message() {
        assert_eq!(
            rendered_errors("p q"),
            ["línea 1, columna 3: se esperaba el fin de la proposición"]
        );
        assert_eq!(
            rendered_errors("p w"),
            ["línea 1, columna 3: se esperaba el fin de la proposición"]
        );
    }

    #[test]
    fn v_in_operand_position_is_still_a_variable() {
        assert_eq!(ast("p & v"), "(∧ p v)");
        assert_eq!(ast("v"), "v");
        match parse_one("p & v") {
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                assert_eq!(
                    *left,
                    Expression::Identifier {
                        name: "p".to_string(),
                    }
                );
                assert!(matches!(operator.kind, crate::tokens::TokenType::And));
                assert_eq!(
                    *right,
                    Expression::Identifier {
                        name: "v".to_string(),
                    }
                );
            }
            other => panic!("expected a binary expression, got {other:?}"),
        }
    }

    #[test]
    fn recovery_reports_the_v_message_and_the_next_line_error() {
        assert_eq!(
            rendered_errors("p v q\nr &"),
            [
                "línea 1, columna 3: 'v' no es un operador; usa '|', '∨' u 'o' para la disyunción",
                "línea 2, columna 4: se esperaba una expresión",
            ]
        );
    }

    // --- Literals and identifiers ------------------------------------------

    #[test]
    fn every_literal_spelling_parses_to_its_truth_value() {
        assert_eq!(ast("V"), "V");
        assert_eq!(ast("1"), "V");
        assert_eq!(ast("F"), "F");
        assert_eq!(ast("0"), "F");
    }

    #[test]
    fn multi_character_identifiers_are_kept_whole() {
        assert_eq!(ast("p1"), "p1");
        assert_eq!(ast("a_b"), "a_b");
        assert_eq!(ast("p1 & a_b"), "(∧ p1 a_b)");
    }

    #[test]
    fn identifiers_are_case_sensitive() {
        let lower = parse_one("p");
        let upper = parse_one("P");

        assert_ne!(lower, upper);
        assert_eq!(ast("p"), "p");
        assert_eq!(ast("P"), "P");
    }
}
