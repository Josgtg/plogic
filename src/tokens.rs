//! The token vocabulary.
//!
//! `TokenType` is the closed set of lexical kinds and [`TokenType::map`] is the
//! only place where their spellings are defined (ADR-003); a [`Token`] adds the
//! position in the source and renders its normalized spelling
//! (`design/tokens.md`, ADR-036).

use std::fmt;

/// A lexical kind, payload included.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenType {
    /// Negation.
    Not,
    /// Conjunction.
    And,
    /// Disjunction.
    Or,
    /// Material implication.
    Implication,
    /// Material equivalence.
    Biconditional,
    /// A truth-value constant.
    Literal(bool),
    /// A propositional variable, by name.
    Identifier(String),
    /// Opening parenthesis.
    LeftParen,
    /// Closing parenthesis.
    RightParen,
    /// The end of a proposition.
    Newline,
    /// The synthetic end of the input.
    Eof,
}

impl TokenType {
    /// The kind a lexeme spells, or the message saying what was expected.
    ///
    /// This is the single place where spellings are defined (ADR-003), so a
    /// scanner never needs a table of its own. An unknown lexeme is rejected
    /// according to its shape: a run of digits is an invalid literal, anything
    /// else is an invalid character. The scanner records that message in a
    /// `ScanError` at the token's position (`design/scanner.md`).
    pub fn map(lexeme: &str) -> Result<TokenType, String> {
        let kind = match lexeme {
            "!" | "¬" | "no" => TokenType::Not,
            "&" | "∧" | "y" => TokenType::And,
            "|" | "∨" | "o" => TokenType::Or,
            "->" | "→" | "entonces" => TokenType::Implication,
            "<->" | "↔" | "sisolosi" => TokenType::Biconditional,
            "1" | "V" => TokenType::Literal(true),
            "0" | "F" => TokenType::Literal(false),
            "(" => TokenType::LeftParen,
            ")" => TokenType::RightParen,
            _ if !lexeme.is_empty() && lexeme.chars().all(|c| c.is_ascii_digit()) => {
                return Err(format!("literal inválido '{lexeme}'"));
            }
            _ => return Err(format!("carácter inválido '{lexeme}'")),
        };
        Ok(kind)
    }
}

/// The normalized spelling of a kind (`design/tokens.md`).
///
/// Only the kinds that can appear in a formula have one: the connectives, the
/// literals, an identifier's name and the parentheses. `Newline` and `Eof` are
/// structural, so they render as the empty string (ADR-036).
impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let spelling = match self {
            TokenType::Not => "¬",
            TokenType::And => "∧",
            TokenType::Or => "∨",
            TokenType::Implication => "→",
            TokenType::Biconditional => "↔",
            TokenType::Literal(true) => "V",
            TokenType::Literal(false) => "F",
            TokenType::Identifier(name) => return f.write_str(name),
            TokenType::LeftParen => "(",
            TokenType::RightParen => ")",
            TokenType::Newline | TokenType::Eof => "",
        };
        f.write_str(spelling)
    }
}

/// A lexical unit: its kind and the 1-based position of its first character.
///
/// `kind` is the Rust name of the design's `type`, which is a keyword. For
/// `Eof` the position is the one just past the end of the source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    pub kind: TokenType,
    pub line: usize,
    pub col: usize,
}

/// A token displays as its kind does.
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::{Token, TokenType};

    fn map(lexeme: &str) -> Result<TokenType, String> {
        TokenType::map(lexeme)
    }

    #[test]
    fn every_spelling_of_a_connective_maps_to_its_kind() {
        let cases = [
            (TokenType::Not, ["!", "¬", "no"]),
            (TokenType::And, ["&", "∧", "y"]),
            (TokenType::Or, ["|", "∨", "o"]),
            (TokenType::Implication, ["->", "→", "entonces"]),
            (TokenType::Biconditional, ["<->", "↔", "sisolosi"]),
        ];
        for (kind, spellings) in cases {
            for spelling in spellings {
                assert_eq!(map(spelling), Ok(kind.clone()), "spelling {spelling:?}");
            }
        }
        assert_eq!(map("("), Ok(TokenType::LeftParen));
        assert_eq!(map(")"), Ok(TokenType::RightParen));
    }

    #[test]
    fn every_literal_spelling_maps_to_its_truth_value() {
        assert_eq!(map("1"), Ok(TokenType::Literal(true)));
        assert_eq!(map("V"), Ok(TokenType::Literal(true)));
        assert_eq!(map("0"), Ok(TokenType::Literal(false)));
        assert_eq!(map("F"), Ok(TokenType::Literal(false)));
    }

    #[test]
    fn a_digit_run_is_an_invalid_literal() {
        assert_eq!(map("12"), Err("literal inválido '12'".to_string()));
        assert_eq!(map("2"), Err("literal inválido '2'".to_string()));
    }

    #[test]
    fn anything_else_is_an_invalid_character() {
        assert_eq!(map("%"), Err("carácter inválido '%'".to_string()));
        assert_eq!(map("ñ"), Err("carácter inválido 'ñ'".to_string()));
        assert_eq!(map("_"), Err("carácter inválido '_'".to_string()));
        assert_eq!(map(""), Err("carácter inválido ''".to_string()));
    }

    #[test]
    fn words_are_case_sensitive() {
        assert_eq!(map("V"), Ok(TokenType::Literal(true)));
        assert_eq!(map("F"), Ok(TokenType::Literal(false)));
        // Lower-case spellings of a literal are identifiers, and only a word
        // the scanner does not know becomes one, so `map` rejects them.
        assert_eq!(map("v"), Err("carácter inválido 'v'".to_string()));
        assert_eq!(map("f"), Err("carácter inválido 'f'".to_string()));
        assert_eq!(map("Y"), Err("carácter inválido 'Y'".to_string()));
        assert_eq!(map("No"), Err("carácter inválido 'No'".to_string()));
    }

    #[test]
    fn map_never_produces_an_identifier_or_a_structural_token() {
        for lexeme in ["p", "P", "p1", "12", "%", "ñ", "_"] {
            let kind = map(lexeme);
            assert!(
                !matches!(
                    kind,
                    Ok(TokenType::Identifier(_)) | Ok(TokenType::Newline) | Ok(TokenType::Eof)
                ),
                "lexeme {lexeme:?} gave {kind:?}"
            );
        }
    }

    #[test]
    fn spellings_are_normalized() {
        let cases = [
            (TokenType::Not, "¬"),
            (TokenType::And, "∧"),
            (TokenType::Or, "∨"),
            (TokenType::Implication, "→"),
            (TokenType::Biconditional, "↔"),
            (TokenType::Literal(true), "V"),
            (TokenType::Literal(false), "F"),
            (TokenType::Identifier("p1".to_string()), "p1"),
            (TokenType::LeftParen, "("),
            (TokenType::RightParen, ")"),
        ];
        for (kind, spelling) in cases {
            assert_eq!(kind.to_string(), spelling);
        }
    }

    #[test]
    fn structural_tokens_have_no_spelling() {
        assert_eq!(TokenType::Newline.to_string(), "");
        assert_eq!(TokenType::Eof.to_string(), "");
    }

    #[test]
    fn a_token_displays_its_kind() {
        let token = Token {
            kind: TokenType::Identifier("q".to_string()),
            line: 2,
            col: 7,
        };
        assert_eq!(token.to_string(), "q");
        assert_eq!(token.line, 2);
        assert_eq!(token.col, 7);
    }
}
