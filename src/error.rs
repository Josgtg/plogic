//! The error values (`design/error_handling.md`).
//!
//! Errors are values: a failure is `Err`, never an exception or a panic, and a
//! failing stage carries a list of them (ADR-019). The `Display` of an error is
//! the line the user reads, so `Orchestrator.report` can stay a thin wrapper
//! (ADR-037).

use thiserror::Error;

/// A user mistake, with its 1-based position when it has one.
///
/// The message is lower-case and says what was expected; `EvalError` is about
/// size, not position, so it carries no `line`/`col`.
// The names are the design's (`error_handling.md`): each variant says which stage
// produced it, so the suffix is the point, not noise.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    /// A lexical error, at the offending character or at the start of the run.
    #[error("línea {line}, columna {col}: {message}")]
    ScanError {
        line: usize,
        col: usize,
        message: String,
    },
    /// A syntax error, at the offending token.
    #[error("línea {line}, columna {col}: {message}")]
    ParseError {
        line: usize,
        col: usize,
        message: String,
    },
    /// A semantic limit, which has no position.
    #[error("{message}")]
    EvalError { message: String },
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn a_positioned_error_displays_as_a_report_line() {
        let error = Error::ScanError {
            line: 2,
            col: 7,
            message: "carácter inválido '%'".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "línea 2, columna 7: carácter inválido '%'"
        );
        let error = Error::ParseError {
            line: 1,
            col: 4,
            message: "se esperaba una expresión".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "línea 1, columna 4: se esperaba una expresión"
        );
    }

    #[test]
    fn a_size_error_displays_as_its_message() {
        let error = Error::EvalError {
            message: "resultado muy largo: más de 4096 filas".to_string(),
        };
        assert_eq!(error.to_string(), "resultado muy largo: más de 4096 filas");
    }
}
