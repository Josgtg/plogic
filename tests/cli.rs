//! End-to-end tests for the wired CLI (`design/testing.md` section 6).
//!
//! Every test runs the real binary and checks what the user sees: stdout,
//! stderr, exit code and, when relevant, the files it writes. `NO_COLOR` and
//! the piped streams keep the output deterministic and plain.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_plogic");

/// A fresh temporary directory unique to the calling test.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plogic-cli-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the temporary directory should be creatable");
    dir
}

/// Runs plogic in `dir` with no standard input.
fn plogic(dir: &Path, args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .output()
        .expect("plogic should start")
}

/// Runs plogic in `dir`, feeding `input` on standard input.
fn plogic_with_stdin(dir: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(BIN)
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("plogic should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(input.as_bytes())
        .expect("the input should be writable");
    child.wait_with_output().expect("plogic should finish")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn code(output: &Output) -> Option<i32> {
    output.status.code()
}

// --- Presentation ---------------------------------------------------------

#[test]
fn default_presentation_shows_simplified_table_and_classification() {
    let dir = temp_dir("default-presentation");
    let output = plogic(&dir, &["p -> q"]);

    assert_eq!(code(&output), Some(0));
    assert_eq!(
        text(&output.stdout),
        "Simplificada: q ∨ ¬p\n\n\
         q  p  | q ∨ ¬p\n\
         F  F  | V\n\
         F  V  | F\n\
         V  F  | V\n\
         V  V  | V\n\n\
         Esta proposición es una contingencia.\n"
    );
    assert!(output.stderr.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_simplificar_omits_the_simplified_line_only() {
    let dir = temp_dir("no-simplificar");
    let output = plogic(&dir, &["--no-simplificar", "p -> q"]);

    assert_eq!(code(&output), Some(0));
    let stdout = text(&output.stdout);
    assert!(!stdout.contains("Simplificada:"));
    assert_eq!(
        stdout,
        "p  q  | p → q\n\
         F  F  | V\n\
         F  V  | V\n\
         V  F  | F\n\
         V  V  | V\n\n\
         Esta proposición es una contingencia.\n"
    );
    // Only the separator between the table and the classification remains.
    assert_eq!(stdout.matches("\n\n").count(), 1);
    assert!(output.stderr.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn default_table_of_a_tautology_has_no_variables() {
    let dir = temp_dir("simplified-tautology");
    let output = plogic(&dir, &["p | !p"]);

    assert_eq!(code(&output), Some(0));
    let stdout = text(&output.stdout);
    assert_eq!(
        stdout,
        "Simplificada: V\n\n\
         V\n\
         V\n\n\
         Esta proposición es una tautología.\n"
    );
    assert!(!stdout.contains('|'));
    assert!(output.stderr.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn simplificar_prints_only_the_formula() {
    let dir = temp_dir("formula-only");
    let output = plogic(&dir, &["-s", "p -> q"]);

    assert_eq!(code(&output), Some(0));
    let stdout = text(&output.stdout);
    assert_eq!(stdout, "Simplificada: q ∨ ¬p\n");
    assert!(!stdout.contains('|'));
    assert!(!stdout.contains("Esta proposición"));
    assert!(output.stderr.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn simplificar_with_no_simplificar_is_a_usage_error() {
    let dir = temp_dir("formula-conflict");
    let output = plogic(&dir, &["--simplificar", "--no-simplificar", "p"]);

    assert_eq!(code(&output), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = text(&output.stderr);
    assert!(
        stderr.starts_with("'--simplificar' no se puede usar con '--no-simplificar'\n\n"),
        "unexpected stderr: {stderr:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_constant_table_has_a_v_header_and_a_v_row_and_no_separator() {
    let dir = temp_dir("constant");
    let output = plogic(&dir, &["-p", "V"]);

    assert_eq!(code(&output), Some(0));
    let stdout = text(&output.stdout);
    assert_eq!(
        stdout,
        "Simplificada: V\n\n\
         V\n\
         V\n\n\
         Esta proposición es una tautología.\n"
    );
    assert!(!stdout.contains('|'));
    assert!(output.stderr.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn multi_character_variables_are_aligned_without_trailing_spaces() {
    let dir = temp_dir("alignment");
    let output = plogic(&dir, &["p1 & p2"]);

    assert_eq!(code(&output), Some(0));
    let stdout = text(&output.stdout);
    assert_eq!(
        stdout,
        "Simplificada: p1 ∧ p2\n\n\
         p1  p2  | p1 ∧ p2\n\
         F   F   | F\n\
         F   V   | F\n\
         V   F   | F\n\
         V   V   | V\n\n\
         Esta proposición es una contingencia.\n"
    );
    for line in stdout.lines() {
        assert_eq!(line, line.trim_end(), "trailing space in {line:?}");
    }
    assert!(output.stderr.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

// --- File mode ------------------------------------------------------------

#[test]
fn file_mode_numbers_each_proposition_and_separates_the_blocks() {
    let dir = temp_dir("file-numbering");
    std::fs::write(dir.join("f.txt"), "p & q\nq | r\n").expect("the input file should be writable");

    let output = plogic(&dir, &["-a", "f.txt"]);

    assert_eq!(code(&output), Some(0));
    let stdout = text(&output.stdout);
    assert!(stdout.starts_with("Proposición 1:\n\nSimplificada: p ∧ q\n\n"));
    assert!(stdout.contains("Proposición 2:\n\nSimplificada: q ∨ r\n\n"));
    // One blank line between the two blocks.
    assert!(stdout.contains("Esta proposición es una contingencia.\n\nProposición 2:"));
    assert_eq!(stdout.matches("Proposición ").count(), 2);
    assert!(output.stderr.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn simplificar_in_file_mode_numbers_each_formula() {
    let dir = temp_dir("file-formula");
    std::fs::write(dir.join("f.txt"), "p & q\nq | r\n").expect("the input file should be writable");

    let output = plogic(&dir, &["-s", "-a", "f.txt"]);

    assert_eq!(code(&output), Some(0));
    assert_eq!(
        text(&output.stdout),
        "Proposición 1:\n\nSimplificada: p ∧ q\n\n\
         Proposición 2:\n\nSimplificada: q ∨ r\n"
    );
    assert!(!text(&output.stdout).contains('|'));
    assert!(output.stderr.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_with_one_proposition_still_gets_a_heading() {
    let dir = temp_dir("file-single");
    std::fs::write(dir.join("f.txt"), "p\n").expect("the input file should be writable");

    let output = plogic(&dir, &["-a", "f.txt"]);

    assert_eq!(code(&output), Some(0));
    assert!(text(&output.stdout).starts_with("Proposición 1:\n\nSimplificada: p\n\n"));
    assert!(output.stderr.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_bare_proposition_gets_no_heading() {
    let dir = temp_dir("bare-no-heading");
    let output = plogic(&dir, &["p"]);

    assert_eq!(code(&output), Some(0));
    assert!(!text(&output.stdout).contains("Proposición "));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn blank_lines_produce_no_table_and_do_not_advance_the_numbering() {
    let dir = temp_dir("blank-lines");
    std::fs::write(dir.join("f.txt"), "p & q\n\nq | r\n")
        .expect("the input file should be writable");

    let output = plogic(&dir, &["-a", "f.txt"]);

    assert_eq!(code(&output), Some(0));
    let stdout = text(&output.stdout);
    assert!(stdout.contains("Proposición 1:\n\nSimplificada: p ∧ q"));
    assert!(stdout.contains("Proposición 2:\n\nSimplificada: q ∨ r"));
    assert_eq!(stdout.matches("Proposición ").count(), 2);
    assert!(output.stderr.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_empty_file_is_rejected() {
    let dir = temp_dir("empty-file");
    std::fs::write(dir.join("empty.txt"), "").expect("the input file should be writable");

    let output = plogic(&dir, &["-a", "empty.txt"]);

    assert_eq!(code(&output), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        text(&output.stderr),
        "el archivo no contiene proposiciones\n"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_blank_only_file_is_rejected() {
    let dir = temp_dir("blank-file");
    std::fs::write(dir.join("blank.txt"), "  \n\n\t\n").expect("the input file should be writable");

    let output = plogic(&dir, &["-a", "blank.txt"]);

    assert_eq!(code(&output), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        text(&output.stderr),
        "el archivo no contiene proposiciones\n"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_unreadable_file_is_reported_with_its_path() {
    let dir = temp_dir("unreadable");
    let output = plogic(&dir, &["-a", "missing.txt"]);

    assert_eq!(code(&output), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = text(&output.stderr);
    assert!(
        stderr.starts_with("no se pudo leer el archivo 'missing.txt'"),
        "unexpected stderr: {stderr:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// --- Output destinations --------------------------------------------------

#[test]
fn salida_writes_the_default_presentation_to_a_file() {
    let dir = temp_dir("salida");
    let console = plogic(&dir, &["p -> q"]);
    assert_eq!(code(&console), Some(0));

    let output = plogic(&dir, &["-o", "out.txt", "p -> q"]);

    assert_eq!(code(&output), Some(0));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    let written = std::fs::read(dir.join("out.txt")).expect("out.txt should exist");
    assert_eq!(text(&written), text(&console.stdout));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn simplificar_composes_with_salida() {
    let dir = temp_dir("formula-salida");
    let output = plogic(&dir, &["-s", "-o", "out.txt", "p -> q"]);

    assert_eq!(code(&output), Some(0));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    let written = std::fs::read(dir.join("out.txt")).expect("out.txt should exist");
    assert_eq!(text(&written), "Simplificada: q ∨ ¬p\n");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failing_run_with_salida_leaves_an_existing_file_untouched() {
    let dir = temp_dir("salida-failure");
    std::fs::write(dir.join("out.txt"), "sentinel").expect("out.txt should be writable");

    let output = plogic(&dir, &["-o", "out.txt", "p &"]);

    assert_eq!(code(&output), Some(1));
    assert_eq!(
        text(&output.stderr),
        "línea 1, columna 4: se esperaba una expresión\n"
    );
    let written = std::fs::read(dir.join("out.txt")).expect("out.txt should still exist");
    assert_eq!(text(&written), "sentinel");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_unwritable_output_is_reported_with_its_path() {
    let dir = temp_dir("salida-unwritable");
    let output = plogic(&dir, &["-o", "missing/out.txt", "p"]);

    assert_eq!(code(&output), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = text(&output.stderr);
    assert!(
        stderr.starts_with("no se pudo escribir el archivo 'missing/out.txt'"),
        "unexpected stderr: {stderr:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// --- Errors ---------------------------------------------------------------

#[test]
fn parse_errors_go_to_stderr_only() {
    let dir = temp_dir("parse-error");
    let output = plogic(&dir, &["p &"]);

    assert_eq!(code(&output), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        text(&output.stderr),
        "línea 1, columna 4: se esperaba una expresión\n"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// --- Automatic redirect ---------------------------------------------------

/// `p1 & p2 & ... & p<count>`, one variable per operand.
fn conjunction(count: usize) -> String {
    (1..=count)
        .map(|index| format!("p{index}"))
        .collect::<Vec<_>>()
        .join(" & ")
}

#[test]
fn a_table_above_the_threshold_redirects_to_plogic_output_txt() {
    let dir = temp_dir("redirect");
    let source = conjunction(11);

    let output = plogic(&dir, &[&source]);

    assert_eq!(code(&output), Some(0));
    assert!(output.stdout.is_empty());
    assert_eq!(
        text(&output.stderr),
        "la salida se escribió en plogic-output.txt\n"
    );
    let written =
        std::fs::read(dir.join("plogic-output.txt")).expect("the redirect file should exist");
    let written = text(&written);
    let header = written
        .lines()
        .find(|line| line.contains("| p1 ∧"))
        .unwrap_or_else(|| panic!("no header line in {written:?}"));
    assert!(header.contains("p10"), "unexpected header: {header:?}");
    let rows = written
        .lines()
        .filter(|line| line.contains('|') && line.starts_with(['F', 'V']))
        .count();
    assert_eq!(rows, 2048);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn exactly_1024_rows_still_go_to_stdout() {
    let dir = temp_dir("threshold");
    let source = conjunction(10);

    let output = plogic(&dir, &[&source]);

    assert_eq!(code(&output), Some(0));
    assert!(output.stderr.is_empty());
    let stdout = text(&output.stdout);
    assert!(stdout.contains("| p1 ∧"), "unexpected stdout: {stdout:?}");
    let rows = stdout
        .lines()
        .filter(|line| line.contains('|') && line.starts_with(['F', 'V']))
        .count();
    assert_eq!(rows, 1024);
    assert!(!dir.join("plogic-output.txt").exists());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn salida_wins_over_the_automatic_redirect() {
    let dir = temp_dir("salida-wins");
    let source = conjunction(11);

    let output = plogic(&dir, &["-o", "out.txt", &source]);

    assert_eq!(code(&output), Some(0));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert!(dir.join("out.txt").exists());
    assert!(!dir.join("plogic-output.txt").exists());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_redirect_threshold_uses_the_tabulated_expression() {
    let dir = temp_dir("threshold-expression");
    let source = "(p1 | !p1) & p2 & p3 & p4 & p5 & p6 & p7 & p8 & p9 & p10 & p11";

    let simplified = plogic(&dir, &[source]);

    assert_eq!(code(&simplified), Some(0));
    assert!(!simplified.stdout.is_empty());
    assert!(simplified.stderr.is_empty());
    assert!(!dir.join("plogic-output.txt").exists());

    let original = plogic(&dir, &["--no-simplificar", source]);

    assert_eq!(code(&original), Some(0));
    assert!(original.stdout.is_empty());
    assert_eq!(
        text(&original.stderr),
        "la salida se escribió en plogic-output.txt\n"
    );
    assert!(dir.join("plogic-output.txt").exists());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_size_limit_uses_the_tabulated_expression_too() {
    let dir = temp_dir("size-limit-expression");
    let source = "(p1 | !p1) & p2 & p3 & p4 & p5 & p6 & p7 & p8 & p9 & p10 & p11 & p12 & p13";

    let simplified = plogic(&dir, &[source]);

    assert_eq!(code(&simplified), Some(0));
    assert!(simplified.stdout.is_empty());
    assert_eq!(
        text(&simplified.stderr),
        "la salida se escribió en plogic-output.txt\n"
    );
    let written =
        std::fs::read(dir.join("plogic-output.txt")).expect("the redirect file should exist");
    let written = text(&written);
    let rows = written
        .lines()
        .filter(|line| line.contains('|') && line.starts_with(['F', 'V']))
        .count();
    assert_eq!(rows, 4096);

    let original = plogic(&dir, &["--no-simplificar", source]);

    assert_eq!(code(&original), Some(1));
    assert!(original.stdout.is_empty());
    assert_eq!(
        text(&original.stderr),
        "resultado muy largo: más de 4096 filas\n"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn simplificar_never_creates_the_redirect_file() {
    let dir = temp_dir("simplificar-no-redirect");
    let source = "p1 | !p1 | p2 | p3 | p4 | p5 | p6 | p7 | p8 | p9 | p10 | p11 | p12 | p13";

    let output = plogic(&dir, &["-s", source]);

    assert_eq!(code(&output), Some(0));
    assert_eq!(text(&output.stdout), "Simplificada: V\n");
    assert!(output.stderr.is_empty());
    assert!(!dir.join("plogic-output.txt").exists());

    let _ = std::fs::remove_dir_all(&dir);
}

// --- Coloring -------------------------------------------------------------

#[test]
fn piped_output_has_no_ansi_escapes() {
    let dir = temp_dir("no-color");
    for args in [
        &["p -> q"][..],
        &["p | !p"][..],
        &["-s", "p -> q"][..],
        &["p &"][..],
    ] {
        let output = plogic(&dir, args);
        assert!(
            !output.stdout.contains(&0x1b),
            "escape in stdout for {args:?}"
        );
        assert!(
            !output.stderr.contains(&0x1b),
            "escape in stderr for {args:?}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

// --- Interactive mode -----------------------------------------------------

#[test]
fn interactive_session_survives_a_parse_error() {
    let dir = temp_dir("interactive-error");

    let output = plogic_with_stdin(&dir, &["-i"], "p\n\nq & r\np &\nexit\n");

    assert_eq!(code(&output), Some(0));
    let stdout = text(&output.stdout);
    assert_eq!(stdout.matches("Esta proposición es").count(), 2);
    assert!(stdout.contains("> "));
    // The session reached the `exit` line after the failed `p &`.
    assert!(stdout.ends_with("> "));
    assert!(text(&output.stderr).contains("línea 1, columna 4: se esperaba una expresión"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn interactive_salir_ends_the_session() {
    let dir = temp_dir("interactive-salir");
    let output = plogic_with_stdin(&dir, &["-i"], "salir\n");

    assert_eq!(code(&output), Some(0));
    assert_eq!(text(&output.stdout), "> ");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn interactive_end_of_input_ends_the_session() {
    let dir = temp_dir("interactive-eof");
    let output = plogic_with_stdin(&dir, &["-i"], "");

    assert_eq!(code(&output), Some(0));
    let stdout = text(&output.stdout);
    assert!(
        stdout.ends_with('\n'),
        "stdout should end with a newline: {stdout:?}"
    );
    assert_eq!(stdout, "> \n");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn interactive_no_simplificar_omits_the_simplified_line() {
    let dir = temp_dir("interactive-no-simplificar");
    let output = plogic_with_stdin(&dir, &["-i", "--no-simplificar"], "p\n");

    assert_eq!(code(&output), Some(0));
    let stdout = text(&output.stdout);
    assert!(!stdout.contains("Simplificada:"));
    assert!(stdout.contains("Esta proposición es una contingencia."));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn interactive_simplificar_prints_only_formulas_per_line() {
    let dir = temp_dir("interactive-formula");
    let output = plogic_with_stdin(&dir, &["-i", "-s"], "p -> q\np | !p\nexit\n");

    assert_eq!(code(&output), Some(0));
    assert_eq!(
        text(&output.stdout),
        "> Simplificada: q ∨ ¬p\n\n> Simplificada: V\n\n> "
    );
    assert!(!text(&output.stdout).contains("Esta proposición"));
    assert!(!text(&output.stdout).contains('|'));
    assert!(output.stderr.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}
