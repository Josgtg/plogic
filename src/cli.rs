//! The command-line interface.
//!
//! The only impure component of the design (`design/ui.md`). Everything the
//! program reads from or writes to the outside world starts here. The parsing
//! of the arguments lives in [`crate::args`].

use std::io::IsTerminal;

/// Whether errors are painted: only when stderr is a terminal, and never when
/// `NO_COLOR` asks for plain text (`design/ui.md`, ADR-035).
pub fn error_colors() -> bool {
    std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}
