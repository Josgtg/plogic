//! The command-line interface.
//!
//! The only impure component of the design (`design/ui.md`). Everything the
//! program reads from or writes to the outside world starts here; the parsing
//! of the arguments lives in [`crate::args`] and the pipeline in
//! [`crate::orchestrator`]. The presentation (the aligned table, the
//! `Simplificada:` line, the optional `Proceso:` list and the classification)
//! is plain formatting, kept in this module because it is the UI's job.

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;

use colored::Colorize;

use crate::args::{self, Config, Input, Simplification};
use crate::error::Error;
use crate::expression::InfixPrinter;
use crate::orchestrator::{Evaluation, Orchestrator};
use crate::simplifier::{Simplification as Simplified, Step};
use crate::truth_table::{Classification, TruthTable};

/// The file an oversized non-interactive run is written to (`design/ui.md`).
const REDIRECT_FILE: &str = "plogic-output.txt";

/// A table with more rows than this is not printed to the console.
const REDIRECT_THRESHOLD: usize = 1 << 10;

/// Whether errors are painted: only when stderr is a terminal, and never when
/// `NO_COLOR` asks for plain text (`design/ui.md`, ADR-035).
pub fn error_colors() -> bool {
    std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

/// Whether the table cells are painted: same rule as [`error_colors`], but
/// decided from stdout, where the tables go.
fn table_colors() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

/// Runs a validated command line and returns the process exit code: 0 on
/// success, 1 on any failing run (`design/ui.md`). Usage errors never reach
/// this function; [`crate::args`] reports them with code 2.
pub fn run(config: Config) -> i32 {
    match &config.input {
        Input::Proposition(proposition) => run_source(proposition.clone(), &config, false),
        Input::File(path) => run_file(path, &config),
        Input::Interactive => run_interactive(&config),
    }
}

/// File mode: reads the file and evaluates it as a batch.
fn run_file(path: &Path, config: &Config) -> i32 {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("no se pudo leer el archivo '{}': {error}", path.display());
            return 1;
        }
    };
    if source.trim().is_empty() {
        eprintln!("el archivo no contiene proposiciones");
        return 1;
    }
    run_source(source, config, true)
}

/// Evaluates one source text (a single proposition or a whole file) and
/// writes its tables, or its simplified formulas with `--simplificar`.
/// `numbered` adds the file-mode `Proposición n:` heading.
fn run_source(source: String, config: &Config, numbered: bool) -> i32 {
    match config.simplification {
        Simplification::Formula => {
            let formulas = match Orchestrator::simplify(source) {
                Ok(formulas) => formulas,
                Err(errors) => return report_errors(&errors),
            };
            let output = render_formulas(&formulas, numbered, config.show_process);
            write_output(&output, config.output.as_deref(), false)
        }
        Simplification::Table | Simplification::Disabled => {
            let simplify = matches!(config.simplification, Simplification::Table);
            let evaluations = match Orchestrator::evaluate(source, simplify) {
                Ok(evaluations) => evaluations,
                Err(errors) => return report_errors(&errors),
            };
            let redirect = config.output.is_none() && oversized(&evaluations);
            let color = config.output.is_none() && !redirect && table_colors();
            let output = render(&evaluations, numbered, color, config.show_process);
            write_output(&output, config.output.as_deref(), redirect)
        }
    }
}

/// Writes the run's presentation: to the `-o` path, to `plogic-output.txt`
/// when a table is oversized, or to stdout.
fn write_output(output: &str, path: Option<&Path>, redirect: bool) -> i32 {
    let destination = match (path, redirect) {
        (Some(path), _) => path,
        (None, true) => Path::new(REDIRECT_FILE),
        (None, false) => {
            let _ = io::stdout().write_all(output.as_bytes());
            return 0;
        }
    };
    if let Err(error) = std::fs::write(destination, output) {
        eprintln!(
            "no se pudo escribir el archivo '{}': {error}",
            destination.display()
        );
        return 1;
    }
    if path.is_none() {
        eprintln!("la salida se escribió en {REDIRECT_FILE}");
    }
    0
}

/// Whether any table of the run is too long to read on the console.
fn oversized(evaluations: &[Evaluation]) -> bool {
    evaluations
        .iter()
        .any(|evaluation| evaluation.table.rows.len() > REDIRECT_THRESHOLD)
}

/// The whole run's text, one block per table, separated by a blank line.
fn render(evaluations: &[Evaluation], numbered: bool, color: bool, show_process: bool) -> String {
    let blocks: Vec<String> = evaluations
        .iter()
        .enumerate()
        .map(|(index, evaluation)| {
            let presented = present(evaluation, color, show_process);
            if numbered {
                format!("Proposición {}:\n\n{presented}", index + 1)
            } else {
                presented
            }
        })
        .collect();
    blocks.join("\n")
}

/// The formula-only presentation of `--simplificar`: one `Simplificada:` line
/// per proposition, separated by a blank line, with the file-mode heading and
/// the optional `Proceso:` list.
fn render_formulas(formulas: &[Simplified], numbered: bool, show_process: bool) -> String {
    let blocks: Vec<String> = formulas
        .iter()
        .enumerate()
        .map(|(index, formula)| {
            let mut block = String::new();
            if numbered {
                block.push_str(&format!("Proposición {}:\n\n", index + 1));
            }
            block.push_str("Simplificada: ");
            block.push_str(&InfixPrinter::print(&formula.expression));
            block.push('\n');
            if show_process && !formula.steps.is_empty() {
                block.push('\n');
                block.push_str(&render_process(&formula.original, &formula.steps));
            }
            block
        })
        .collect();
    blocks.join("\n")
}

/// One table's presentation: the simplified formula when the Simplifier ran,
/// its process when `-m` asked for it, the aligned truth table, and the
/// classification, with a blank line between items (ADR-029, ADR-047).
fn present(evaluation: &Evaluation, color: bool, show_process: bool) -> String {
    let table = &evaluation.table;
    let mut output = String::new();
    if let Some(simplified) = &table.simplified {
        output.push_str("Simplificada: ");
        output.push_str(simplified);
        output.push_str("\n\n");
    }
    if show_process
        && let Some(simplification) = &evaluation.simplification
        && !simplification.steps.is_empty()
    {
        output.push_str(&render_process(&simplification.original, &simplification.steps));
        output.push('\n');
    }
    output.push_str("Tabla de verdad:\n");
    output.push_str(&render_table(table, color));
    output.push_str("\n\n");
    output.push_str("Clasificación: ");
    output.push_str(classification(table));
    output.push('\n');
    output
}

/// The `Proceso:` block: one numbered line per applied law, with the canonical
/// formula it produced.
fn render_process(original: &str, steps: &[Step]) -> String {
    let mut output = String::from("Proceso:\n");
    output.push_str(&format!("0. Original: {}\n", original));
    for (index, step) in steps.iter().enumerate() {
        output.push_str(&format!(
            "{}. {}: {}\n",
            index + 1,
            step.rule.label(),
            InfixPrinter::print(&step.expression)
        ));
    }
    output
}

/// The aligned table: the variable columns, the `|` separator and the result
/// column. Cells are left-aligned to their column width and the last column
/// carries no trailing padding; a table without variables has no `|`.
fn render_table(table: &TruthTable, color: bool) -> String {
    let widths: Vec<usize> = table
        .variables
        .iter()
        .map(|name| name.chars().count())
        .collect();
    let mut lines = Vec::with_capacity(table.rows.len() + 1);

    let mut header = String::new();
    for (name, width) in table.variables.iter().zip(&widths) {
        header.push_str(name);
        header.push_str(&" ".repeat(width - name.chars().count()));
        header.push_str("  ");
    }
    if !table.variables.is_empty() {
        header.push_str("| ");
    }
    header.push_str(&table.label);
    lines.push(header);

    for row in &table.rows {
        let mut line = String::new();
        for (value, width) in row.values.iter().zip(&widths) {
            let cell = if *value { "V" } else { "F" };
            line.push_str(&painted(cell, *value, color));
            line.push_str(&" ".repeat(width - 1));
            line.push_str("  ");
        }
        if !table.variables.is_empty() {
            line.push_str("| ");
        }
        line.push_str(&painted(
            if row.result { "V" } else { "F" },
            row.result,
            color,
        ));
        lines.push(line);
    }
    lines.join("\n")
}

/// Paints one `V`/`F` cell green or red when the output is a terminal.
fn painted(cell: &str, value: bool, color: bool) -> String {
    if !color {
        return cell.to_string();
    }
    colored::control::set_override(true);
    let painted = if value {
        cell.green().to_string()
    } else {
        cell.red().to_string()
    };
    colored::control::unset_override();
    painted
}

/// The classification line (`design/ui.md`, ADR-021).
fn classification(table: &TruthTable) -> &'static str {
    match table.classify() {
        Classification::Tautology => "Esta proposición es una tautología.",
        Classification::Contradiction => "Esta proposición es una contradicción.",
        Classification::Contingency => "Esta proposición es una contingencia.",
    }
}

/// Prints a failing stage's errors on stderr, one per line, and returns 1.
fn report_errors(errors: &[Error]) -> i32 {
    let color = error_colors();
    for error in errors {
        let line = Orchestrator::report(error.clone());
        let line = if color { args::bright_red(&line) } else { line };
        eprintln!("{line}");
    }
    1
}

/// The read-eval-print session: one proposition per line, presented as it
/// arrives. `exit`, `salir` and end-of-input end it; a blank line only shows a
/// new prompt (`design/ui.md`).
fn run_interactive(config: &Config) -> i32 {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    loop {
        print!("> ");
        let _ = io::stdout().flush();
        let Some(line) = lines.next() else {
            println!();
            return 0;
        };
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("no se pudo leer la entrada: {error}");
                return 1;
            }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "exit" || trimmed == "salir" {
            return 0;
        }
        match config.simplification {
            Simplification::Formula => match Orchestrator::simplify(line) {
                Ok(formulas) => {
                    print!("{}", render_formulas(&formulas, false, config.show_process));
                }
                Err(errors) => {
                    report_errors(&errors);
                }
            },
            Simplification::Table | Simplification::Disabled => {
                let simplify = matches!(config.simplification, Simplification::Table);
                match Orchestrator::evaluate(line, simplify) {
                    Ok(evaluations) => {
                        let color = table_colors();
                        for evaluation in &evaluations {
                            print!("{}", present(evaluation, color, config.show_process));
                        }
                    }
                    Err(errors) => {
                        report_errors(&errors);
                    }
                }
            }
        }
        println!();
    }
}
