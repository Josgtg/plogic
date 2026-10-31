//! Command-line parsing.
//!
//! clap reads the tokens, renders the help screen and handles `-h`/`--help` and
//! `-V`/`--version` (ADR-032, ADR-033). Every error message the user sees comes
//! from plogic: the exact Spanish strings and the fixed precedence are accepted
//! design (ADR-022, ADR-024, ADR-027, ADR-028, ADR-031).

use std::ffi::OsString;
use std::path::PathBuf;

use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{CommandFactory, FromArgMatches, Parser};
use colored::Colorize;

/// The usage line the help screen prints (`design/ui.md`).
pub const USAGE: &str = "uso: plogic [opciones] [proposición]";

/// Heading of the positional section of the help screen.
const ARGUMENTS_HEADING: &str = "argumentos";

/// Heading of the option section of the help screen.
const OPTIONS_HEADING: &str = "opciones";

// The command line as clap parses it: one field per option, exactly as typed.
// `Args::into_config` applies the plogic rules on top of it. The help screen
// comes from these attributes: `about` for the first line, one doc comment per
// argument, `value_name` for its value, `help_heading` for its section. Neither
// the struct nor the fields carry a `///` comment, because clap derive would
// turn it into `long_about`/`long_help` and `--help` would render a different
// screen than `-h`.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "plogic",
    bin_name = "plogic",
    version,
    about = "plogic: evalúa proposiciones de lógica proposicional",
    override_usage = USAGE,
    help_template = "{about-with-newline}\n{usage}\n\n{all-args}",
    next_line_help = false
)]
pub struct Args {
    /// evalúa una proposición
    #[arg(value_name = "proposición", help_heading = ARGUMENTS_HEADING)]
    pub texto: Option<String>,

    /// evalúa una proposición
    #[arg(
        short = 'p',
        long = "proposicion",
        value_name = "texto",
        help_heading = OPTIONS_HEADING
    )]
    pub proposicion: Option<String>,

    /// evalúa una proposición por línea
    #[arg(
        short = 'a',
        long = "archivo",
        value_name = "archivo",
        help_heading = OPTIONS_HEADING
    )]
    pub archivo: Option<String>,

    /// inicia una sesión interactiva
    #[arg(short = 'i', long = "interactivo", help_heading = OPTIONS_HEADING)]
    pub interactivo: bool,

    /// no simplifica la proposición
    #[arg(long = "no-simplificar", help_heading = OPTIONS_HEADING)]
    pub no_simplificar: bool,

    /// construye la tabla con la expresión simplificada
    #[arg(long = "tabla-simplificada", help_heading = OPTIONS_HEADING)]
    pub tabla_simplificada: bool,

    /// escribe la salida en <archivo>
    #[arg(
        short = 's',
        long = "salida",
        value_name = "archivo",
        help_heading = OPTIONS_HEADING
    )]
    pub salida: Option<String>,
}

/// The clap command, with the Spanish descriptions of the two built-in flags.
/// clap adds `--help` and `--version` while it builds, so they can only be
/// touched afterwards.
fn clap_command() -> clap::Command {
    let mut command = Args::command();
    command.build();
    command
        .mut_arg("help", |arg| {
            arg.help("muestra esta ayuda")
                .long_help("muestra esta ayuda")
                .help_heading(OPTIONS_HEADING)
        })
        .mut_arg("version", |arg| {
            arg.help("muestra la versión")
                .long_help("muestra la versión")
                .help_heading(OPTIONS_HEADING)
        })
}

/// The help screen `-h`/`--help` prints, ending with a newline.
pub fn help() -> String {
    clap_command().render_help().to_string()
}

/// The version line `-V`/`--version` prints, ending with a newline.
#[cfg(test)]
fn version() -> String {
    clap_command().render_version()
}

/// Where a run takes its propositions from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// One proposition, exactly as it was written on the command line.
    Proposition(String),
    /// A file with one proposition per line.
    File(PathBuf),
    /// A read-eval-print session.
    Interactive,
}

/// A validated run configuration: what the rest of the program consumes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub input: Input,
    /// False with `--no-simplificar`.
    pub simplify: bool,
    pub table_simplified: bool,
    /// `-s` / `--salida`.
    pub output: Option<PathBuf>,
}

/// A usage error: the CLI prints [`UsageError::message`], a blank line and the
/// full help screen on stderr, then exits 2. The order in which these are
/// checked is the fixed precedence of ADR-027.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageError {
    /// `falta el valor de '<flag>'`
    MissingValue(String),
    /// `opción desconocida: '<flag>'`
    UnknownFlag(String),
    /// `solo se puede usar un modo a la vez`
    SeveralModes,
    /// `argumento inesperado: '<arg>'`
    UnexpectedArgument(String),
    /// `no se proporcionó ninguna proposición`
    NoProposition,
    /// `'--tabla-simplificada' no se puede usar con '--no-simplificar'`
    TableSimplifiedWithNoSimplify,
    /// `'-s' no se puede usar con '--interactivo'`
    OutputWithInteractive,
}

impl UsageError {
    /// The specific message, lower-case, without the usage line.
    pub fn message(&self) -> String {
        match self {
            UsageError::MissingValue(flag) => format!("falta el valor de '{flag}'"),
            UsageError::UnknownFlag(flag) => format!("opción desconocida: '{flag}'"),
            UsageError::SeveralModes => "solo se puede usar un modo a la vez".to_string(),
            UsageError::UnexpectedArgument(arg) => format!("argumento inesperado: '{arg}'"),
            UsageError::NoProposition => "no se proporcionó ninguna proposición".to_string(),
            UsageError::TableSimplifiedWithNoSimplify => {
                "'--tabla-simplificada' no se puede usar con '--no-simplificar'".to_string()
            }
            UsageError::OutputWithInteractive => {
                "'-s' no se puede usar con '--interactivo'".to_string()
            }
        }
    }

    /// What the CLI prints on stderr: the specific message, a blank line and
    /// the full help screen, without its trailing newline. With `color` the
    /// message is bright red and the help screen stays plain (ADR-035).
    pub fn report(&self, color: bool) -> String {
        let message = self.message();
        let message = if color { bright_red(&message) } else { message };
        format!("{message}\n\n{}", help().trim_end())
    }
}

/// Paints text bright red. `colored` decides whether to emit the escapes by
/// looking at *stdout*, but errors go to stderr, so the caller makes that
/// decision ([`crate::cli::error_colors`]) and it is forced here.
fn bright_red(text: &str) -> String {
    colored::control::set_override(true);
    let painted = text.bright_red().to_string();
    colored::control::unset_override();
    painted
}

impl Args {
    /// Applies the plogic rules to the parsed command line, in the order
    /// ADR-027 fixes: modes, extra positional, missing proposition,
    /// `--tabla-simplificada`, `-s` with `--interactivo`.
    pub fn into_config(self) -> Result<Config, UsageError> {
        let modes = [
            self.proposicion.is_some(),
            self.archivo.is_some(),
            self.interactivo,
        ];
        if modes.iter().filter(|chosen| **chosen).count() > 1 {
            return Err(UsageError::SeveralModes);
        }

        if let Some(bare) = &self.texto
            && modes.iter().any(|chosen| *chosen)
        {
            return Err(UsageError::UnexpectedArgument(bare.clone()));
        }

        let input = if self.interactivo {
            Input::Interactive
        } else if let Some(archivo) = &self.archivo {
            Input::File(PathBuf::from(archivo))
        } else if let Some(text) = self.proposicion.as_ref().or(self.texto.as_ref()) {
            // ADR-024: only the emptiness check trims; the scanner gets the
            // original text.
            if text.trim().is_empty() {
                return Err(UsageError::NoProposition);
            }
            Input::Proposition(text.clone())
        } else {
            return Err(UsageError::NoProposition);
        };

        if self.tabla_simplificada && self.no_simplificar {
            return Err(UsageError::TableSimplifiedWithNoSimplify);
        }
        if self.salida.is_some() && self.interactivo {
            return Err(UsageError::OutputWithInteractive);
        }

        Ok(Config {
            input,
            simplify: !self.no_simplificar,
            table_simplified: self.tabla_simplificada,
            output: self.salida.map(PathBuf::from),
        })
    }
}

/// Parses the arguments that follow the program name.
///
/// `-h`/`--help` and `-V`/`--version` are clap's: it prints them to stdout and
/// exits 0, in argument order (ADR-033). Every other failure becomes one of
/// plogic's usage errors.
pub fn parse<I, T>(raw: I) -> Result<Config, UsageError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let raw: Vec<OsString> = raw.into_iter().map(Into::into).collect();

    // ADR-024: every argument that begins with `-` is an option, `-` included,
    // which clap would otherwise accept as a value.
    if lone_dash_is_option(&raw) {
        return Err(UsageError::UnknownFlag("-".to_string()));
    }

    let argv = std::iter::once(OsString::from("plogic")).chain(raw.iter().cloned());
    match clap_command().try_get_matches_from(argv) {
        Ok(matches) => Args::from_arg_matches(&matches)
            .map_err(|error| usage_error(&error, &raw))?
            .into_config(),
        // A clustered `-hV`, for instance, arrives here.
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            error.exit()
        }
        Err(error) => Err(usage_error(&error, &raw)),
    }
}

/// Long name and short form of every option of [`Args`]. Used to echo a flag
/// the way the user typed it.
const FLAGS: &[(&str, Option<char>)] = &[
    ("proposicion", Some('p')),
    ("archivo", Some('a')),
    ("interactivo", Some('i')),
    ("no-simplificar", None),
    ("tabla-simplificada", None),
    ("salida", Some('s')),
];

/// The tokens before the first `--`, as lossy strings.
fn option_tokens(raw: &[OsString]) -> impl Iterator<Item = String> + '_ {
    raw.iter()
        .map(|token| token.to_string_lossy().into_owned())
        .take_while(|token| token != "--")
}

/// True when a lone `-` is used as an option: before `--` and not the value of
/// an option that still wants one.
fn lone_dash_is_option(raw: &[OsString]) -> bool {
    let mut wants_value = false;
    for token in option_tokens(raw) {
        if wants_value {
            wants_value = false;
            continue;
        }
        if token == "-" {
            return true;
        }
        wants_value = consumes_next_token(&token);
    }
    false
}

/// True when clap would take the following token as this option's value.
fn consumes_next_token(token: &str) -> bool {
    if let Some(rest) = token.strip_prefix("--") {
        return !rest.contains('=') && takes_value(rest);
    }
    short_flag_without_value(token).is_some()
}

/// The short options that take a value: `-p`, `-a` and `-s`.
fn takes_value(name: &str) -> bool {
    short_of(name).is_some_and(|flag| matches!(flag, 'p' | 'a' | 's'))
}

/// For a short-option token (`-s`, or a cluster such as `-is`), the short flag
/// that would take the next token as its value.
fn short_flag_without_value(token: &str) -> Option<char> {
    let rest = token.strip_prefix('-')?;
    if rest.is_empty() || rest.starts_with('-') {
        return None;
    }
    for (index, flag) in rest.char_indices() {
        if matches!(flag, 'p' | 'a' | 's') {
            return (index + flag.len_utf8() == rest.len()).then_some(flag);
        }
    }
    None
}

/// The short form of an option, by long name or by short name.
fn short_of(name: &str) -> Option<char> {
    FLAGS
        .iter()
        .find(|(long, short)| *long == name || short.is_some_and(|c| c.to_string() == name))
        .and_then(|(_, short)| *short)
}

/// Maps a clap error onto the plogic message for it.
fn usage_error(error: &clap::Error, raw: &[OsString]) -> UsageError {
    let invalid = invalid_arg(error);
    match error.kind() {
        ErrorKind::UnknownArgument => match invalid {
            // A dash-prefixed token in option position is an option.
            Some(arg) if arg.starts_with('-') => UsageError::UnknownFlag(arg),
            Some(arg) => UsageError::UnexpectedArgument(arg),
            None => UsageError::UnexpectedArgument(String::new()),
        },
        // clap reports a flag left without its value as an invalid value.
        ErrorKind::InvalidValue => {
            let long = invalid.as_deref().and_then(long_name).unwrap_or_default();
            UsageError::MissingValue(typed_flag(long, raw))
        }
        // A repeated flag: mode flags are another mode, the rest is an
        // argument the user cannot pass twice (ADR-031).
        ErrorKind::ArgumentConflict => {
            let long = invalid.as_deref().and_then(long_name).unwrap_or_default();
            if matches!(long, "proposicion" | "archivo" | "interactivo") {
                UsageError::SeveralModes
            } else {
                UsageError::UnexpectedArgument(typed_flag(long, raw))
            }
        }
        _ => UsageError::UnexpectedArgument(error.to_string()),
    }
}

fn invalid_arg(error: &clap::Error) -> Option<String> {
    match error.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(text)) => Some(text.clone()),
        _ => None,
    }
}

/// `--proposicion <texto>` -> `proposicion`.
fn long_name(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("--")?;
    Some(rest.split([' ', '=']).next().unwrap_or(rest))
}

/// The way the user wrote the flag: `-p` if a short form appears, otherwise
/// `--proposicion`.
fn typed_flag(long: &str, raw: &[OsString]) -> String {
    let short = short_of(long).map(|flag| flag.to_string());
    let mut typed = format!("--{long}");
    for token in option_tokens(raw) {
        if token == typed {
            return typed;
        }
        if let Some(flag) = short.as_deref()
            && short_flag_without_value(&token).is_some_and(|c| c.to_string() == flag)
        {
            typed = format!("-{flag}");
        }
    }
    typed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> Result<Config, UsageError> {
        parse(args.iter().copied())
    }

    fn config(args: &[&str]) -> Config {
        match run(args) {
            Ok(config) => config,
            Err(error) => panic!("{args:?} should be a run, got {error:?}"),
        }
    }

    fn error(args: &[&str]) -> UsageError {
        match run(args) {
            Err(error) => error,
            other => panic!("{args:?} should fail, got {other:?}"),
        }
    }

    fn proposition(text: &str) -> Input {
        Input::Proposition(text.to_string())
    }

    #[test]
    fn bare_argument_is_a_proposition() {
        assert_eq!(
            config(&["p -> q"]),
            Config {
                input: proposition("p -> q"),
                simplify: true,
                table_simplified: false,
                output: None,
            }
        );
    }

    #[test]
    fn proposition_keeps_surrounding_spaces_and_long_form_works() {
        assert_eq!(config(&["-p", "  p  "]).input, proposition("  p  "));
        assert_eq!(config(&["--proposicion", "p"]).input, proposition("p"));
        assert_eq!(config(&["-p=p"]).input, proposition("p"));
        assert_eq!(config(&["--proposicion=p"]).input, proposition("p"));
    }

    #[test]
    fn file_and_interactive_modes() {
        assert_eq!(
            config(&["-a", "f.txt"]).input,
            Input::File(PathBuf::from("f.txt"))
        );
        assert_eq!(
            config(&["--archivo", "f.txt"]).input,
            Input::File(PathBuf::from("f.txt"))
        );
        assert_eq!(config(&["-i"]).input, Input::Interactive);
        assert_eq!(config(&["--interactivo"]).input, Input::Interactive);
    }

    #[test]
    fn modifiers_combine() {
        let parsed = config(&["-p", "p", "--no-simplificar", "-s", "out.txt"]);
        assert!(!parsed.simplify);
        assert!(!parsed.table_simplified);
        assert_eq!(parsed.output, Some(PathBuf::from("out.txt")));

        let parsed = config(&["p", "--tabla-simplificada", "--salida", "out.txt"]);
        assert!(parsed.simplify);
        assert!(parsed.table_simplified);
        assert_eq!(parsed.output, Some(PathBuf::from("out.txt")));

        assert!(!config(&["-i", "--no-simplificar"]).simplify);
        assert!(config(&["-i", "--tabla-simplificada"]).table_simplified);
    }

    #[test]
    fn short_flags_cluster_and_take_attached_values() {
        // `-is out.txt` parses as both `-i` and `-s`, which the design refuses
        // before it reaches a run.
        assert_eq!(
            error(&["-is", "out.txt"]),
            UsageError::OutputWithInteractive
        );
        assert_eq!(
            config(&["-sp", "q"]),
            Config {
                input: proposition("q"),
                simplify: true,
                table_simplified: false,
                output: Some(PathBuf::from("p")),
            }
        );
    }

    #[test]
    fn separator_ends_option_parsing() {
        assert_eq!(config(&["--", "->"]).input, proposition("->"));
        assert_eq!(config(&["-p", "p", "--"]).input, proposition("p"));
        assert_eq!(config(&["--", "--help"]).input, proposition("--help"));
        assert_eq!(config(&["--", "-q"]).input, proposition("-q"));
    }

    #[test]
    fn trailing_separator_without_proposition_is_a_usage_error() {
        assert_eq!(error(&["--"]), UsageError::NoProposition);
    }

    #[test]
    fn unknown_flags_are_reported_as_typed() {
        assert_eq!(error(&["-x"]), UsageError::UnknownFlag("-x".to_string()));
        assert_eq!(error(&["->"]), UsageError::UnknownFlag("->".to_string()));
        assert_eq!(error(&["-"]), UsageError::UnknownFlag("-".to_string()));
        assert_eq!(
            error(&["-x", "-i", "-s", "out.txt"]),
            UsageError::UnknownFlag("-x".to_string())
        );
    }

    #[test]
    fn missing_values_name_the_typed_flag() {
        assert_eq!(error(&["-p"]), UsageError::MissingValue("-p".to_string()));
        assert_eq!(
            error(&["--proposicion"]),
            UsageError::MissingValue("--proposicion".to_string())
        );
        assert_eq!(error(&["-s"]), UsageError::MissingValue("-s".to_string()));
        assert_eq!(
            error(&["-p", "--", "p"]),
            UsageError::MissingValue("-p".to_string())
        );
        assert_eq!(
            error(&["-p", "-s", "x"]),
            UsageError::MissingValue("-p".to_string())
        );
        assert_eq!(error(&["-is"]), UsageError::MissingValue("-s".to_string()));
    }

    #[test]
    fn missing_value_wins_over_the_extra_positional() {
        assert_eq!(
            error(&["-p", "--", "-x"]),
            UsageError::MissingValue("-p".to_string())
        );
    }

    #[test]
    fn repeated_flags_reuse_existing_messages() {
        assert_eq!(error(&["-p", "p", "-p", "q"]), UsageError::SeveralModes);
        assert_eq!(error(&["-i", "-i"]), UsageError::SeveralModes);
        assert_eq!(error(&["-a", "f", "-a", "g"]), UsageError::SeveralModes);
        assert_eq!(
            error(&["-s", "a", "-s", "b"]),
            UsageError::UnexpectedArgument("-s".to_string())
        );
        assert_eq!(
            error(&["--no-simplificar", "--no-simplificar"]),
            UsageError::UnexpectedArgument("--no-simplificar".to_string())
        );
    }

    #[test]
    fn two_modes_at_once() {
        assert_eq!(error(&["-p", "p", "-i"]), UsageError::SeveralModes);
        assert_eq!(error(&["-p", "p", "-a", "f"]), UsageError::SeveralModes);
        assert_eq!(error(&["-a", "f", "-i"]), UsageError::SeveralModes);
    }

    #[test]
    fn extra_positional_argument() {
        assert_eq!(
            error(&["p", "q"]),
            UsageError::UnexpectedArgument("q".to_string())
        );
        assert_eq!(
            error(&["-p", "p", "q"]),
            UsageError::UnexpectedArgument("q".to_string())
        );
        assert_eq!(
            error(&["--", "a", "b"]),
            UsageError::UnexpectedArgument("b".to_string())
        );
    }

    #[test]
    fn nothing_to_evaluate() {
        assert_eq!(error(&[]), UsageError::NoProposition);
        assert_eq!(error(&["--no-simplificar"]), UsageError::NoProposition);
        assert_eq!(
            error(&["--tabla-simplificada", "-s", "out.txt"]),
            UsageError::NoProposition
        );
        assert_eq!(error(&[""]), UsageError::NoProposition);
        assert_eq!(error(&["   "]), UsageError::NoProposition);
        assert_eq!(error(&["-p", ""]), UsageError::NoProposition);
        assert_eq!(error(&["-p", "  "]), UsageError::NoProposition);
    }

    #[test]
    fn conflicting_modifiers() {
        assert_eq!(
            error(&["p", "--tabla-simplificada", "--no-simplificar"]),
            UsageError::TableSimplifiedWithNoSimplify
        );
        assert_eq!(
            error(&["-s", "out.txt", "-i"]),
            UsageError::OutputWithInteractive
        );
    }

    #[test]
    fn precedence_puts_table_conflict_before_output_conflict() {
        assert_eq!(
            error(&[
                "-i",
                "-s",
                "out.txt",
                "--tabla-simplificada",
                "--no-simplificar"
            ]),
            UsageError::TableSimplifiedWithNoSimplify
        );
    }

    #[test]
    fn precedence_puts_unknown_flag_first() {
        assert_eq!(
            error(&[
                "-i",
                "-s",
                "out.txt",
                "--tabla-simplificada",
                "--no-simplificar",
                "-x"
            ]),
            UsageError::UnknownFlag("-x".to_string())
        );
        assert_eq!(
            error(&["-x", "-p", "p", "-p", "q"]),
            UsageError::UnknownFlag("-x".to_string())
        );
    }

    #[test]
    fn clap_handles_help_and_version_in_argument_order() {
        // ADR-033: clap prints these itself. Asking `parse` would end the test
        // process, so the check stops at the clap error.
        let kind = |args: &[&str]| {
            let argv = std::iter::once("plogic").chain(args.iter().copied());
            clap_command()
                .try_get_matches_from(argv)
                .unwrap_err()
                .kind()
        };
        assert_eq!(kind(&["-h"]), ErrorKind::DisplayHelp);
        assert_eq!(kind(&["--help"]), ErrorKind::DisplayHelp);
        assert_eq!(kind(&["-V"]), ErrorKind::DisplayVersion);
        assert_eq!(kind(&["--version"]), ErrorKind::DisplayVersion);
        assert_eq!(kind(&["-hV"]), ErrorKind::DisplayHelp);
        assert_eq!(kind(&["--help", "--version"]), ErrorKind::DisplayHelp);
        assert_eq!(kind(&["-V", "--help"]), ErrorKind::DisplayVersion);
    }

    #[test]
    fn earlier_usage_errors_beat_help_and_version() {
        assert_eq!(
            error(&["-x", "--help"]),
            UsageError::UnknownFlag("-x".to_string())
        );
        assert_eq!(
            error(&["-p", "--help"]),
            UsageError::MissingValue("-p".to_string())
        );
        assert_eq!(
            error(&["-x", "-V"]),
            UsageError::UnknownFlag("-x".to_string())
        );
    }

    #[test]
    fn help_screen_comes_from_the_argument_attributes() {
        assert_eq!(
            help(),
            "\
plogic: evalúa proposiciones de lógica proposicional

uso: plogic [opciones] [proposición]

argumentos:
  [proposición]  evalúa una proposición

opciones:
  -p, --proposicion <texto>  evalúa una proposición
  -a, --archivo <archivo>    evalúa una proposición por línea
  -i, --interactivo          inicia una sesión interactiva
      --no-simplificar       no simplifica la proposición
      --tabla-simplificada   construye la tabla con la expresión simplificada
  -s, --salida <archivo>     escribe la salida en <archivo>
  -h, --help                 muestra esta ayuda
  -V, --version              muestra la versión
"
        );
    }

    #[test]
    fn version_is_the_package_name_and_version() {
        assert_eq!(version(), "plogic 0.1.0\n");
    }

    #[test]
    fn messages_are_exact() {
        assert_eq!(
            error(&["-p"]).report(false),
            format!("falta el valor de '-p'\n\n{}", help().trim_end())
        );
        assert_eq!(error(&["-x"]).message(), "opción desconocida: '-x'");
        assert_eq!(
            error(&["-p", "p", "-i"]).message(),
            "solo se puede usar un modo a la vez"
        );
        assert_eq!(error(&["p", "q"]).message(), "argumento inesperado: 'q'");
        assert_eq!(
            error(&[]).message(),
            "no se proporcionó ninguna proposición"
        );
        assert_eq!(
            error(&["p", "--tabla-simplificada", "--no-simplificar"]).message(),
            "'--tabla-simplificada' no se puede usar con '--no-simplificar'"
        );
        assert_eq!(
            error(&["-s", "out.txt", "-i"]).message(),
            "'-s' no se puede usar con '--interactivo'"
        );
    }

    #[test]
    fn the_cause_is_bright_red_and_the_help_stays_plain() {
        assert_eq!(
            error(&["-p"]).report(true),
            format!(
                "\u{1b}[91mfalta el valor de '-p'\u{1b}[0m\n\n{}",
                help().trim_end()
            )
        );
    }
}
