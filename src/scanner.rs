//! The scanner: source text to tokens (`design/scanner.md`).
//!
//! The first pipeline stage and the only code that reads raw source text. It is
//! pure: the immutable [`ScanState`] is threaded through the private helpers and
//! returned anew (ADR-001). Every spelling is resolved by [`TokenType::map`], so
//! the scanner owns no spelling table (ADR-003); what it owns is the character
//! handling around it.

use crate::error::Error;
use crate::tokens::{Token, TokenType};

/// The immutable state threaded through the helpers: the source, the start of
/// the lexeme being scanned, the position being read, the line, and everything
/// collected so far.
///
/// The state owns the source: nothing after scanning needs the raw text (the
/// table is labelled from the parsed expression), so the caller hands the
/// string over and no helper carries a lifetime (ADR-038).
///
/// The column is not stored: it is derived from the source and `token_start`
/// (ADR-016). Offsets are byte offsets into `source`; every move goes character
/// by character, so a formal symbol is never split.
#[derive(Debug, Clone)]
struct ScanState {
    source: String,
    /// Where the current lexeme starts.
    token_start: usize,
    /// Where the next character to read is.
    current: usize,
    /// The 1-based line of the current position.
    line: usize,
    tokens: Vec<Token>,
    errors: Vec<Error>,
}

/// Tokenizes `source`: `Ok(tokens)`, ending in `Eof`, or `Err(errors)` with
/// every lexical error found in the pass. It takes the text by value, since
/// nothing after scanning needs it (ADR-038).
///
/// This is a stage gate: when anything failed the partial token list is
/// discarded, so a run either continues with a complete list or stops here
/// (ADR-002).
pub fn scan(source: String) -> Result<Vec<Token>, Vec<Error>> {
    let mut state = ScanState {
        source,
        token_start: 0,
        current: 0,
        line: 1,
        tokens: Vec::new(),
        errors: Vec::new(),
    };
    while !is_at_end(&state) {
        state = scan_token(state);
    }
    let state = add_token(state, TokenType::Eof);
    if state.errors.is_empty() {
        Ok(state.tokens)
    } else {
        Err(state.errors)
    }
}

/// Handles the character at the current position: one lexeme, one separator or
/// one line break, and the state that follows.
fn scan_token(state: ScanState) -> ScanState {
    match peek(&state) {
        Some(' ') | Some('\t') => skip(state),
        Some('\n') => line_break(advance(state)),
        // CRLF is one line break, and it is a break at the position of the CR.
        Some('\r') if peek_next(&state) == Some('\n') => line_break(advance(advance(state))),
        Some('(') => add_token(advance(state), TokenType::LeftParen),
        Some(')') => add_token(advance(state), TokenType::RightParen),
        Some('-') => {
            let (found, state) = match_char(advance(state), '>');
            if found {
                add_token(state, TokenType::Implication)
            } else {
                error(state, "se esperaba '>' después de '-'")
            }
        }
        Some('<') => {
            let (found, state) = match_char(advance(state), '-');
            let (found, state) = if found {
                match_char(state, '>')
            } else {
                (false, state)
            };
            if found {
                add_token(state, TokenType::Biconditional)
            } else {
                error(state, "se esperaba '->' después de '<'")
            }
        }
        Some(c) if c.is_ascii_digit() => digits(state),
        Some(c) if c.is_ascii_alphabetic() => word(state),
        // Any other character is a one-character lexeme: `map` decides whether
        // it is a connective (the formal symbols included) or reports
        // `carácter inválido` (ADR-036, ADR-038).
        Some(_) => map_lexeme(advance(state)),
        None => state,
    }
}

/// Records the `Newline` of a line break that has just been consumed. The token
/// sits on the line it ends, so the line advances after the token is added.
fn line_break(state: ScanState) -> ScanState {
    let state = add_token(state, TokenType::Newline);
    let line = state.line + 1;
    ScanState { line, ..state }
}

/// Drops a separator, which produces no token.
fn skip(state: ScanState) -> ScanState {
    let state = advance(state);
    ScanState {
        token_start: state.current,
        ..state
    }
}

/// Scans a run of digits; `map` accepts `0` and `1` and rejects any other run.
fn digits(state: ScanState) -> ScanState {
    map_lexeme(advance_while(state, |c| c.is_ascii_digit()))
}

/// Scans a word: a reserved spelling maps to its kind, anything else is an
/// identifier, which is case-sensitive (ADR-003). An underscore only
/// continues a word; it never starts one (ADR-039).
fn word(state: ScanState) -> ScanState {
    let state = advance_while(state, |c| c.is_ascii_alphanumeric() || c == '_');
    let lexeme = &state.source[state.token_start..state.current];
    let kind = TokenType::map(lexeme).unwrap_or_else(|_| TokenType::Identifier(lexeme.to_string()));
    add_token(state, kind)
}

/// Resolves the current lexeme through the single spelling table: a kind
/// becomes a token and a rejection becomes a `ScanError` at the lexeme's start.
fn map_lexeme(state: ScanState) -> ScanState {
    let lexeme = &state.source[state.token_start..state.current];
    match TokenType::map(lexeme) {
        Ok(kind) => add_token(state, kind),
        Err(message) => error(state, &message),
    }
}

/// The character at `current`, if the source has one left.
fn peek(state: &ScanState) -> Option<char> {
    state.source[state.current..].chars().next()
}

/// The character after the one at `current`.
fn peek_next(state: &ScanState) -> Option<char> {
    state.source[state.current..].chars().nth(1)
}

/// Whether the whole source has been read.
fn is_at_end(state: &ScanState) -> bool {
    state.current >= state.source.len()
}

/// Consumes the character at `current`, whatever its width in bytes.
fn advance(state: ScanState) -> ScanState {
    let width = peek(&state).map_or(0, char::len_utf8);
    let current = state.current + width;
    ScanState { current, ..state }
}

/// Consumes `expected` if it is the next character, and reports whether it was.
fn match_char(state: ScanState, expected: char) -> (bool, ScanState) {
    if peek(&state) == Some(expected) {
        (true, advance(state))
    } else {
        (false, state)
    }
}

/// Consumes the longest run of characters `accept` takes.
fn advance_while(state: ScanState, accept: impl Fn(char) -> bool) -> ScanState {
    let mut state = state;
    while let Some(c) = peek(&state) {
        if !accept(c) {
            break;
        }
        state = advance(state);
    }
    state
}

/// Adds a token of `kind` at the start of the current lexeme and starts a new
/// lexeme after it.
fn add_token(state: ScanState, kind: TokenType) -> ScanState {
    let ScanState {
        source,
        token_start,
        current,
        line,
        mut tokens,
        errors,
    } = state;
    let token = Token {
        kind,
        line,
        col: column(&source, token_start),
    };
    tokens.push(token);
    ScanState {
        source,
        token_start: current,
        current,
        line,
        tokens,
        errors,
    }
}

/// Records a `ScanError` at the start of the current lexeme and starts a new
/// lexeme after it, so scanning goes on.
fn error(state: ScanState, message: &str) -> ScanState {
    let ScanState {
        source,
        token_start,
        current,
        line,
        tokens,
        mut errors,
    } = state;
    errors.push(Error::ScanError {
        line,
        col: column(&source, token_start),
        message: message.to_string(),
    });
    ScanState {
        source,
        token_start: current,
        current,
        line,
        tokens,
        errors,
    }
}

/// The 1-based column of `token_start`: the characters since the last line
/// break, with a tab and a formal symbol counting as one each (ADR-016).
fn column(source: &str, token_start: usize) -> usize {
    source[..token_start]
        .chars()
        .rev()
        .take_while(|&c| c != '\n')
        .count()
        + 1
}

#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::tokens::{Token, TokenType};

    /// `scan` takes the source by value; the tests always have a `&str`.
    fn scan(source: &str) -> Result<Vec<Token>, Vec<Error>> {
        super::scan(source.to_string())
    }

    fn token(kind: TokenType, line: usize, col: usize) -> Token {
        Token { kind, line, col }
    }

    fn id(name: &str) -> TokenType {
        TokenType::Identifier(name.to_string())
    }

    fn kinds(source: &str) -> Vec<TokenType> {
        scan(source)
            .expect("the source should scan")
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    fn scan_error(line: usize, col: usize, message: &str) -> Error {
        Error::ScanError {
            line,
            col,
            message: message.to_string(),
        }
    }

    #[test]
    fn the_product_example_tokenizes() {
        assert_eq!(
            kinds("p | q & s -> t <-> q | 0"),
            vec![
                id("p"),
                TokenType::Or,
                id("q"),
                TokenType::And,
                id("s"),
                TokenType::Implication,
                id("t"),
                TokenType::Biconditional,
                id("q"),
                TokenType::Or,
                TokenType::Literal(false),
                TokenType::Eof,
            ]
        );
    }

    #[test]
    fn spellings_can_be_mixed() {
        assert_eq!(
            kinds("p | q & s entonces t sisolosi q | 0"),
            kinds("p ∨ q ∧ s → t ↔ q ∨ F")
        );
        assert_eq!(
            kinds("no p y q"),
            vec![
                TokenType::Not,
                id("p"),
                TokenType::And,
                id("q"),
                TokenType::Eof,
            ]
        );
        assert_eq!(kinds("1"), vec![TokenType::Literal(true), TokenType::Eof]);
    }

    #[test]
    fn arrows_are_multi_character_lexemes() {
        let implication = vec![id("p"), TokenType::Implication, id("q"), TokenType::Eof];
        assert_eq!(kinds("p->q"), implication);
        assert_eq!(kinds("p -> q"), implication);
        let biconditional = vec![id("p"), TokenType::Biconditional, id("q"), TokenType::Eof];
        assert_eq!(kinds("p<->q"), biconditional);
        assert_eq!(kinds("p <-> q"), biconditional);
    }

    #[test]
    fn a_lone_arrow_part_is_an_error() {
        assert_eq!(
            scan("p - q"),
            Err(vec![scan_error(1, 3, "se esperaba '>' después de '-'")])
        );
        assert_eq!(
            scan("p < q"),
            Err(vec![scan_error(1, 3, "se esperaba '->' después de '<'")])
        );
        assert_eq!(
            scan("1 -"),
            Err(vec![scan_error(1, 3, "se esperaba '>' después de '-'")])
        );
        assert_eq!(
            scan("1 <-"),
            Err(vec![scan_error(1, 3, "se esperaba '->' después de '<'")])
        );
    }

    #[test]
    fn positions_are_one_based_and_counted_in_characters() {
        assert_eq!(
            scan("p").unwrap(),
            vec![token(id("p"), 1, 1), token(TokenType::Eof, 1, 2)]
        );
        assert_eq!(
            scan("¬p").unwrap(),
            vec![
                token(TokenType::Not, 1, 1),
                token(id("p"), 1, 2),
                token(TokenType::Eof, 1, 3),
            ]
        );
        assert_eq!(
            scan("¬ p").unwrap(),
            vec![
                token(TokenType::Not, 1, 1),
                token(id("p"), 1, 3),
                token(TokenType::Eof, 1, 4),
            ]
        );
        assert_eq!(
            scan("\tp").unwrap(),
            vec![token(id("p"), 1, 2), token(TokenType::Eof, 1, 3)]
        );
    }

    #[test]
    fn positions_advance_with_the_line() {
        assert_eq!(
            scan("p\nq").unwrap(),
            vec![
                token(id("p"), 1, 1),
                token(TokenType::Newline, 1, 2),
                token(id("q"), 2, 1),
                token(TokenType::Eof, 2, 2),
            ]
        );
        assert_eq!(
            scan("p\r\nq").unwrap(),
            vec![
                token(id("p"), 1, 1),
                token(TokenType::Newline, 1, 2),
                token(id("q"), 2, 1),
                token(TokenType::Eof, 2, 2),
            ]
        );
    }

    #[test]
    fn line_breaks_are_tokens() {
        assert_eq!(
            kinds("p\nq\n"),
            vec![
                id("p"),
                TokenType::Newline,
                id("q"),
                TokenType::Newline,
                TokenType::Eof,
            ]
        );
        assert_eq!(
            kinds("p\r\nq"),
            vec![id("p"), TokenType::Newline, id("q"), TokenType::Eof]
        );
        assert_eq!(
            kinds("p\n\nq"),
            vec![
                id("p"),
                TokenType::Newline,
                TokenType::Newline,
                id("q"),
                TokenType::Eof,
            ]
        );
    }

    #[test]
    fn a_lone_carriage_return_is_an_invalid_character() {
        assert_eq!(
            scan("p\rq"),
            Err(vec![scan_error(1, 2, "carácter inválido '\r'")])
        );
    }

    #[test]
    fn an_empty_source_is_just_eof() {
        assert_eq!(scan("").unwrap(), vec![token(TokenType::Eof, 1, 1)]);
        assert_eq!(scan(" \t ").unwrap(), vec![token(TokenType::Eof, 1, 4)]);
    }

    #[test]
    fn identifiers_are_case_sensitive() {
        assert_eq!(
            kinds("p P v f V F"),
            vec![
                id("p"),
                id("P"),
                id("v"),
                id("f"),
                TokenType::Literal(true),
                TokenType::Literal(false),
                TokenType::Eof,
            ]
        );
        assert_eq!(kinds("p1 q9"), vec![id("p1"), id("q9"), TokenType::Eof]);
    }

    #[test]
    fn errors_are_collected_and_the_tokens_are_discarded() {
        assert_eq!(
            scan("p % q % r"),
            Err(vec![
                scan_error(1, 3, "carácter inválido '%'"),
                scan_error(1, 7, "carácter inválido '%'"),
            ])
        );
    }

    #[test]
    fn a_run_of_digits_is_one_invalid_literal() {
        assert_eq!(
            scan("12"),
            Err(vec![scan_error(1, 1, "literal inválido '12'")])
        );
        assert_eq!(
            kinds("0 1"),
            vec![
                TokenType::Literal(false),
                TokenType::Literal(true),
                TokenType::Eof,
            ]
        );
    }

    #[test]
    fn an_underscore_continues_an_identifier_but_never_starts_one() {
        assert_eq!(kinds("p_1 a_b"), vec![id("p_1"), id("a_b"), TokenType::Eof]);
        assert_eq!(
            scan("_p"),
            Err(vec![scan_error(1, 1, "carácter inválido '_'")])
        );
        assert_eq!(kinds("p__q"), vec![id("p__q"), TokenType::Eof]);
    }

    #[test]
    fn non_ascii_characters_outside_a_literal_or_word_are_errors() {
        for (source, message) in [
            ("ñ", "carácter inválido 'ñ'"),
            ("ä", "carácter inválido 'ä'"),
            ("_", "carácter inválido '_'"),
        ] {
            assert_eq!(scan(source), Err(vec![scan_error(1, 1, message)]));
        }
    }

    #[test]
    fn errors_report_the_line_they_happen_on() {
        assert_eq!(
            scan("p\n%"),
            Err(vec![scan_error(2, 1, "carácter inválido '%'")])
        );
    }
}
