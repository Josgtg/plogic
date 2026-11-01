//! The graphical window.
//!
//! Bare `plogic` (no arguments at all) opens this window; every argument keeps
//! the command-line interface (`design/gui.md`, ADR-048). The GUI is another
//! impure UI component: it drives the same [`Orchestrator`] as the CLI and
//! builds its own components from the returned data. The pipeline stays pure.

use std::collections::HashSet;
use std::ffi::OsString;

use eframe::egui;
use egui::text::{CCursor, CCursorRange};

use crate::args::UsageError;
use crate::expression::InfixPrinter;
use crate::orchestrator::{Evaluation, Orchestrator};
use crate::scanner::scan;
use crate::simplifier::Step;
use crate::tokens::TokenType;
use crate::truth_table::{Classification, TruthTable};

/// Printed on stderr when the window cannot open, before exiting 1.
pub const STARTUP_ERROR: &str = "no se pudo iniciar la interfaz gráfica";

/// The dark background of the window.
const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(9, 9, 12);
/// The background of the formula field.
const INPUT_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(16, 16, 20);
/// The background of the result panel.
const RESULT_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(12, 12, 15);
/// The background of a result card.
const CARD_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(17, 17, 21);
/// The background of the table header.
const HEADER_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(24, 24, 30);
/// The background of every other table row.
const ROW_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(20, 20, 25);
/// The background of the error card.
const ERROR_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(30, 14, 16);
/// The border of the error card.
const ERROR_BORDER: egui::Color32 = egui::Color32::from_rgb(120, 50, 55);
/// The normal text color.
const TEXT: egui::Color32 = egui::Color32::from_rgb(232, 232, 236);
/// Secondary text (hints and card titles).
const MUTED: egui::Color32 = egui::Color32::from_rgb(148, 148, 158);
/// The border of the framed controls.
const BORDER: egui::Color32 = egui::Color32::from_rgb(56, 56, 64);
/// The background of a hovered widget.
const HOVER: egui::Color32 = egui::Color32::from_rgb(28, 28, 34);
/// The selection background.
const SELECTION: egui::Color32 = egui::Color32::from_rgb(60, 60, 72);
/// The color of an error line and of an `F` cell.
const ERROR: egui::Color32 = egui::Color32::from_rgb(255, 120, 120);
/// The color of a true result and of a tautology.
const TRUE_COLOR: egui::Color32 = egui::Color32::from_rgb(120, 220, 150);
/// The color of a false result and of a contradiction.
const FALSE_COLOR: egui::Color32 = egui::Color32::from_rgb(240, 120, 120);

/// The help is deliberately a little smaller than the body text.
const HELP_FONT: f32 = 12.5;
/// The column headers of the connector list.
const HELP_SMALL: f32 = 11.0;
/// The two help headings.
const HELP_HEADING: f32 = 13.5;
/// The table cells and their header.
const TABLE_FONT: f32 = 13.0;
/// A result card's title.
const CARD_TITLE: f32 = 13.0;
/// The horizontal space between the variable columns and the result column.
const SEPARATOR_WIDTH: f32 = 14.0;
/// The horizontal padding of a table cell, on each side.
const CELL_PADDING: f32 = 10.0;

/// The buttons that insert a connective or a parenthesis at the cursor.
const CONNECTIVES: &[(&str, &str)] = &[
    ("¬", "negación"),
    ("∧", "conjunción"),
    ("∨", "disyunción"),
    ("→", "implicación"),
    ("↔", "bicondicional"),
    ("(", "abre paréntesis"),
    (")", "cierra paréntesis"),
];

/// One connector or literal in the help list: formal symbol, Spanish name and
/// the other accepted spellings.
type HelpTriplet = (&'static str, &'static str, &'static str);

/// One help row: the left group and, optionally, the right group. The two
/// groups keep the list compact while still showing one item per row.
struct HelpRow {
    left: HelpTriplet,
    right: Option<HelpTriplet>,
}

const CONNECTIVE_HELP: &[HelpRow] = &[
    HelpRow {
        left: ("¬", "negación", "!, no"),
        right: Some(("∧", "conjunción", "&, y")),
    },
    HelpRow {
        left: ("∨", "disyunción", "|, o"),
        right: Some(("→", "implicación", "->, entonces")),
    },
    HelpRow {
        left: ("↔", "bicondicional", "<->, sisolosi"),
        right: Some(("V", "verdadero", "1")),
    },
    HelpRow {
        left: ("F", "falso", "0"),
        right: None,
    },
];

/// Whether a run with these arguments opens the window. Only a completely
/// empty argument list does: any flag, proposition or `--` keeps the CLI.
pub fn should_launch(argv: &[OsString]) -> bool {
    argv.is_empty()
}

/// Opens the window. The caller turns an error into [`STARTUP_ERROR`] and
/// exit 1; the error itself is not user-facing.
pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 960.0])
            .with_min_inner_size([760.0, 720.0])
            .with_title("plogic"),
        ..Default::default()
    };
    eframe::run_native(
        "plogic",
        options,
        Box::new(|cc| Ok(Box::new(PlogicApp::new(cc)))),
    )
}

/// The state of the window: the formula, the three controls, the last result
/// and the last known cursor range of the formula field.
struct PlogicApp {
    input: String,
    simplify: bool,
    show_table: bool,
    show_process: bool,
    outcome: Option<Outcome>,
    cursor: Option<CCursorRange>,
}

/// What the result panel shows: either the last evaluation or the errors that
/// replaced it. A new evaluation overwrites the whole value.
enum Outcome {
    Evaluations(Vec<Evaluation>),
    Errors(Vec<String>),
}

impl Default for PlogicApp {
    fn default() -> Self {
        Self {
            input: String::new(),
            simplify: true,
            show_table: true,
            show_process: false,
            outcome: None,
            cursor: None,
        }
    }
}

impl PlogicApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure(&cc.egui_ctx);
        Self::default()
    }

    /// The whole window, from top to bottom (`design/gui.md`).
    fn contents(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new("plogic")
                    .monospace()
                    .size(34.0)
                    .strong(),
            );
            ui.add_space(8.0);
        });

        ui.label(
            egui::RichText::new("Ayuda")
                .monospace()
                .strong()
                .size(HELP_HEADING),
        );
        ui.label(
            egui::RichText::new(
                "Evalúa proposiciones de lógica proposicional y construye su tabla de verdad.",
            )
            .monospace()
            .size(HELP_FONT),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Conectivas y literales")
                .monospace()
                .strong()
                .size(HELP_HEADING),
        );
        connectors_help(ui);
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Usa paréntesis para agrupar.")
                .monospace()
                .size(HELP_SMALL)
                .color(MUTED),
        );

        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Opciones")
                .monospace()
                .strong()
                .size(16.0),
        );
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.simplify, "Simplificar");
            ui.checkbox(&mut self.show_table, "Mostrar tabla");
            ui.checkbox(&mut self.show_process, "Mostrar proceso");
        });

        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Proposición")
                .monospace()
                .strong()
                .size(16.0),
        );
        let output = egui::TextEdit::singleline(&mut self.input)
            .id(input_id())
            .hint_text("p | q & s -> t")
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .min_size(egui::vec2(0.0, 40.0))
            .margin(egui::Margin::symmetric(14, 10))
            .background_color(INPUT_BACKGROUND)
            .text_color(TEXT)
            .show(ui);
        self.cursor = output.cursor_range;
        let enter =
            output.response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));

        ui.label(
            egui::RichText::new("Escribe una proposición y pulsa Evaluar.")
                .monospace()
                .size(12.0)
                .color(MUTED),
        );
        ui.add_space(6.0);
        self.toolbar(ui);
        ui.add_space(8.0);

        let mut evaluate = false;
        ui.vertical_centered(|ui| {
            let button = egui::Button::new(
                egui::RichText::new("Evaluar")
                    .monospace()
                    .size(17.0)
                    .strong(),
            )
            .min_size(egui::vec2(160.0, 40.0));
            if ui.add(button).clicked() {
                evaluate = true;
            }
        });
        if evaluate || enter {
            self.evaluate();
        }

        self.result_section(ui);
        ui.add_space(24.0);
    }

    /// The row of buttons that insert symbols at the cursor, plus the variable
    /// button and the clear button.
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for &(symbol, description) in CONNECTIVES {
                let response = ui
                    .button(egui::RichText::new(symbol).monospace().size(16.0))
                    .on_hover_text(description);
                if response.clicked() {
                    self.insert_at_cursor(ui.ctx(), symbol);
                }
            }

            let response = ui
                .button(egui::RichText::new("variable").monospace())
                .on_hover_text("inserta la siguiente variable (p, q, r…)");
            if response.clicked() {
                let variable = next_variable(&self.input);
                self.insert_at_cursor(ui.ctx(), &variable);
            }

            let response = ui
                .button(egui::RichText::new("Limpiar").monospace())
                .on_hover_text("borra la proposición");
            if response.clicked() {
                self.input.clear();
                self.cursor = None;
            }
        });
    }

    /// Inserts `text` at the last known cursor range, replacing a selection.
    /// Without a known cursor it appends, which is what a first click on a
    /// toolbar button does.
    fn insert_at_cursor(&mut self, ctx: &egui::Context, text: &str) {
        let characters = self.input.chars().count();
        let (start, end) = match self.cursor {
            Some(range) => {
                let [min, max] = range.sorted_cursors();
                (
                    usize::from(min.index).min(characters),
                    usize::from(max.index).min(characters),
                )
            }
            None => (characters, characters),
        };
        let end = end.max(start);
        let start_byte = byte_index(&self.input, start);
        let end_byte = byte_index(&self.input, end);
        self.input.replace_range(start_byte..end_byte, text);

        let new_index = start + text.chars().count();
        let range = CCursorRange::one(CCursor::new(new_index));
        self.cursor = Some(range);
        let id = input_id();
        let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
        state.cursor.set_char_range(Some(range));
        egui::TextEdit::store_state(ctx, id, state);
        ctx.memory_mut(|memory| memory.request_focus(id));
    }

    /// Evaluates the field, replacing whatever was shown before. An empty
    /// field reports the same message the CLI uses for a missing proposition.
    fn evaluate(&mut self) {
        if self.input.trim().is_empty() {
            self.outcome = Some(Outcome::Errors(vec![no_proposition()]));
            return;
        }
        match Orchestrator::evaluate(self.input.clone(), self.simplify) {
            Ok(evaluations) if evaluations.is_empty() => {
                self.outcome = Some(Outcome::Errors(vec![no_proposition()]));
            }
            Ok(evaluations) => {
                self.outcome = Some(Outcome::Evaluations(evaluations));
            }
            Err(errors) => {
                self.outcome = Some(Outcome::Errors(
                    errors.into_iter().map(Orchestrator::report).collect(),
                ));
            }
        }
    }

    /// The result components, appended to the main column. There is no output
    /// box: the whole window scrolls and each card is part of the flow. The
    /// previous result stays until `Evaluar` replaces it.
    fn result_section(&self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        let Some(outcome) = &self.outcome else {
            ui.label(
                egui::RichText::new("El resultado aparecerá aquí.")
                    .monospace()
                    .size(12.0)
                    .color(MUTED),
            );
            return;
        };

        ui.label(
            egui::RichText::new("Resultado")
                .monospace()
                .strong()
                .size(16.0),
        );
        ui.add_space(8.0);
        match outcome {
            Outcome::Errors(errors) => error_card(ui, errors),
            Outcome::Evaluations(evaluations) => {
                for (index, evaluation) in evaluations.iter().enumerate() {
                    if index > 0 {
                        ui.add_space(12.0);
                    }
                    self.evaluation(ui, evaluation);
                }
            }
        }
    }

    /// One evaluation as four components: the formula, the optional process
    /// list, the truth table and the classification.
    fn evaluation(&self, ui: &mut egui::Ui, evaluation: &Evaluation) {
        let table = &evaluation.table;
        let formula = table.simplified.as_deref().unwrap_or(table.label.as_str());
        card(ui, formula_title(table), |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(formula)
                        .monospace()
                        .size(18.0)
                        .color(TEXT),
                )
                .selectable(true)
                .wrap(),
            );
        });

        if self.show_process
            && let Some(simplification) = &evaluation.simplification
            && !simplification.steps.is_empty()
        {
            ui.add_space(8.0);
            card(ui, "Proceso", |ui| {
                process_list(ui, &simplification.steps);
            });
        }

        if self.show_table {
            ui.add_space(8.0);
            card(ui, "Tabla de verdad", |ui| {
                table_component(ui, table);
            });
        }

        ui.add_space(8.0);
        card(ui, "Clasificación", |ui| {
            classification_component(ui, table);
        });
    }
}

impl eframe::App for PlogicApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BACKGROUND)
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("plogic-window")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let total = ui.available_width();
                        let width = total.min(860.0);
                        let side = ((total - width) * 0.5).max(0.0);
                        ui.horizontal(|ui| {
                            ui.add_space(side);
                            ui.vertical(|ui| {
                                ui.set_width(width);
                                self.contents(ui);
                            });
                        });
                    });
            });
    }
}

/// The connector and literal help: a three-column list with the formal symbol,
/// the Spanish name and the other accepted spellings.
fn connectors_help(ui: &mut egui::Ui) {
    egui::Grid::new("plogic-connectors")
        .num_columns(7)
        .spacing([14.0, 2.0])
        .show(ui, |ui| {
            help_headers(ui);
            help_header(ui, "");
            help_headers(ui);
            ui.end_row();

            for row in CONNECTIVE_HELP {
                help_triplet(ui, row.left.0, row.left.1, row.left.2);
                ui.label("");
                if let Some((right_symbol, right_name, right_spellings)) = row.right {
                    help_triplet(ui, right_symbol, right_name, right_spellings);
                } else {
                    ui.label("");
                    ui.label("");
                    ui.label("");
                }
                ui.end_row();
            }
        });
}

fn help_headers(ui: &mut egui::Ui) {
    help_header(ui, "Símbolo");
    help_header(ui, "Nombre");
    help_header(ui, "También");
}

fn help_header(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .monospace()
            .size(HELP_SMALL)
            .color(MUTED),
    );
}

fn help_triplet(ui: &mut egui::Ui, symbol: &str, name: &str, spellings: &str) {
    ui.label(
        egui::RichText::new(symbol)
            .monospace()
            .strong()
            .size(HELP_FONT),
    );
    ui.label(egui::RichText::new(name).monospace().size(HELP_FONT));
    ui.label(
        egui::RichText::new(spellings)
            .monospace()
            .size(HELP_FONT)
            .color(MUTED),
    );
}

/// A rounded card with a small title and its body.
fn card(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD_BACKGROUND)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(title)
                    .monospace()
                    .strong()
                    .size(CARD_TITLE)
                    .color(MUTED),
            );
            ui.add_space(4.0);
            body(ui);
        });
}

/// One row per applied law, as a numbered list.
fn process_list(ui: &mut egui::Ui, steps: &[Step]) {
    for (index, step) in steps.iter().enumerate() {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(format!("{}. {}:", index + 1, step.rule.label()))
                    .monospace()
                    .size(14.0),
            );
            ui.label(
                egui::RichText::new(InfixPrinter::print(&step.expression))
                    .monospace()
                    .size(14.0),
            );
        });
    }
}

/// The error card: one selectable red line per report.
fn error_card(ui: &mut egui::Ui, errors: &[String]) {
    egui::Frame::new()
        .fill(ERROR_BACKGROUND)
        .stroke(egui::Stroke::new(1.0, ERROR_BORDER))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Error")
                    .monospace()
                    .strong()
                    .size(CARD_TITLE)
                    .color(ERROR),
            );
            ui.add_space(4.0);
            for error in errors {
                ui.add(
                    egui::Label::new(egui::RichText::new(error.as_str()).monospace().color(ERROR))
                        .selectable(true),
                );
            }
        });
}

/// A real truth table: a fixed header, a variable column per variable, a
/// separator and the result column. The body is virtualized, and the whole
/// table scrolls horizontally when the columns do not fit.
fn table_component(ui: &mut egui::Ui, table: &TruthTable) {
    let font_id = egui::FontId::monospace(TABLE_FONT);
    let char_width = ui.ctx().fonts_mut(|fonts| fonts.glyph_width(&font_id, '0'));
    let widths: Vec<f32> = table
        .variables
        .iter()
        .map(|name| cell_width(name.chars().count(), char_width))
        .collect();
    let result_width = cell_width("Resultado".chars().count(), char_width);
    let has_variables = !table.variables.is_empty();
    let table_width = widths.iter().sum::<f32>()
        + if has_variables { SEPARATOR_WIDTH } else { 0.0 }
        + result_width;
    let row_height = (TABLE_FONT + 10.0).max(ui.text_style_height(&egui::TextStyle::Monospace));

    egui::ScrollArea::horizontal()
        .id_salt("plogic-table-horizontal")
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(table_width);

                egui::Frame::new().fill(HEADER_BACKGROUND).show(ui, |ui| {
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        ui.horizontal(|ui| {
                            for (name, width) in table.variables.iter().zip(&widths) {
                                table_cell(ui, name, *width, row_height, TEXT, true);
                            }
                            if has_variables {
                                table_cell(ui, "|", SEPARATOR_WIDTH, row_height, MUTED, false);
                            }
                            table_cell(ui, "Resultado", result_width, row_height, TEXT, true);
                        });
                    });
                });

                ui.separator();

                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (index, row) in table.rows.iter().enumerate() {
                        let fill = if index % 2 == 0 {
                            ROW_BACKGROUND
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        egui::Frame::new().fill(fill).show(ui, |ui| {
                            ui.scope(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                                ui.horizontal(|ui| {
                                    for (value, width) in row.values.iter().zip(&widths) {
                                        let (text, color) = value_cell(*value);
                                        table_cell(ui, text, *width, row_height, color, false);
                                    }
                                    if has_variables {
                                        table_cell(
                                            ui,
                                            "|",
                                            SEPARATOR_WIDTH,
                                            row_height,
                                            MUTED,
                                            false,
                                        );
                                    }
                                    let (text, color) = value_cell(row.result);
                                    table_cell(ui, text, result_width, row_height, color, false);
                                });
                            });
                        });
                    }
                });
            });
        });
}

fn cell_width(characters: usize, char_width: f32) -> f32 {
    characters.max(1) as f32 * char_width + CELL_PADDING * 2.0
}

fn table_cell(
    ui: &mut egui::Ui,
    text: &str,
    width: f32,
    height: f32,
    color: egui::Color32,
    strong: bool,
) {
    let mut rich = egui::RichText::new(text)
        .monospace()
        .size(TABLE_FONT)
        .color(color);
    if strong {
        rich = rich.strong();
    }
    ui.add_sized(
        [width, height],
        egui::Label::new(rich).halign(egui::Align::Center),
    );
}

fn value_cell(value: bool) -> (&'static str, egui::Color32) {
    if value {
        ("V", TRUE_COLOR)
    } else {
        ("F", FALSE_COLOR)
    }
}

fn formula_title(table: &TruthTable) -> &'static str {
    if table.simplified.is_some() {
        "Simplificada"
    } else {
        "Original"
    }
}

fn classification_display(table: &TruthTable) -> (&'static str, egui::Color32) {
    match table.classify() {
        Classification::Tautology => ("Esta proposición es una tautología.", TRUE_COLOR),
        Classification::Contradiction => ("Esta proposición es una contradicción.", FALSE_COLOR),
        Classification::Contingency => ("Esta proposición es una contingencia.", MUTED),
    }
}

fn classification_component(ui: &mut egui::Ui, table: &TruthTable) {
    let (sentence, color) = classification_display(table);
    ui.label(
        egui::RichText::new(sentence)
            .monospace()
            .size(15.0)
            .color(color),
    );
}

/// Applies the dark, monospace look of the mockup. Ubuntu-Light is already
/// embedded by egui and is added as a fallback for the logic symbols that the
/// monospace face may not carry.
fn configure(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    if let Some(monospace) = fonts.families.get_mut(&egui::FontFamily::Monospace)
        && !monospace.iter().any(|family| family == "Ubuntu-Light")
    {
        let at = monospace.len().min(1);
        monospace.insert(at, "Ubuntu-Light".to_owned());
    }
    ctx.set_fonts(fonts);

    let mut style = (*ctx.global_style()).clone();
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = BACKGROUND;
    visuals.extreme_bg_color = INPUT_BACKGROUND;
    visuals.faint_bg_color = RESULT_BACKGROUND;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = SELECTION;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.weak_bg_fill = INPUT_BACKGROUND;
    visuals.widgets.hovered.weak_bg_fill = HOVER;
    style.visuals = visuals;
    style.text_styles = [
        (
            egui::TextStyle::Heading,
            egui::FontId::new(26.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Body,
            egui::FontId::new(15.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Monospace,
            egui::FontId::new(14.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Button,
            egui::FontId::new(15.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Small,
            egui::FontId::new(12.0, egui::FontFamily::Monospace),
        ),
    ]
    .into_iter()
    .collect();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size.y = 30.0;
    ctx.set_global_style(style);
}

/// The next single-letter variable that the input does not use yet, from `p`
/// to `z`, skipping `y`: `y` is the conjunction word and can never be an
/// identifier. `o` is before `p` and is not a candidate. `p` is returned when
/// every usable letter is taken; an invalid formula also falls back to `p`
/// because the scanner cannot list its identifiers.
fn next_variable(input: &str) -> String {
    let used: HashSet<String> = scan(input.to_string())
        .map(|tokens| {
            tokens
                .into_iter()
                .filter_map(|token| match token.kind {
                    TokenType::Identifier(name) => Some(name),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    ('p'..='z')
        .filter(|letter| *letter != 'y')
        .map(|letter| letter.to_string())
        .find(|name| !used.contains(name))
        .unwrap_or_else(|| "p".to_string())
}

/// Converts a character index (the cursor counts characters) into a byte index
/// for `String::replace_range`.
fn byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

fn input_id() -> egui::Id {
    egui::Id::unique("plogic-proposition")
}

fn no_proposition() -> String {
    UsageError::NoProposition.message()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simplifier::Rule;

    fn arguments(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    fn displayed_table(source: &str, simplify: bool) -> TruthTable {
        let mut evaluations = Orchestrator::evaluate(source.to_string(), simplify)
            .unwrap_or_else(|errors| panic!("source {source:?} failed: {errors:?}"));
        assert_eq!(evaluations.len(), 1, "source {source:?}");
        evaluations.remove(0).table
    }

    fn evaluations(app: &PlogicApp) -> &[Evaluation] {
        match &app.outcome {
            Some(Outcome::Evaluations(evaluations)) => evaluations,
            Some(Outcome::Errors(errors)) => {
                panic!("expected evaluations, got errors: {errors:?}")
            }
            None => panic!("expected evaluations, got no outcome"),
        }
    }

    fn errors(app: &PlogicApp) -> &[String] {
        match &app.outcome {
            Some(Outcome::Errors(errors)) => errors,
            Some(Outcome::Evaluations(evaluations)) => {
                panic!("expected errors, got {} evaluations", evaluations.len())
            }
            None => panic!("expected errors, got no outcome"),
        }
    }

    fn one_evaluation(app: &PlogicApp) -> &Evaluation {
        let evaluations = evaluations(app);
        assert_eq!(evaluations.len(), 1);
        &evaluations[0]
    }

    fn app_with(source: &str) -> PlogicApp {
        let mut app = PlogicApp {
            input: source.to_string(),
            ..Default::default()
        };
        app.evaluate();
        app
    }

    // --- Entry point -------------------------------------------------------

    #[test]
    fn should_launch_is_true_only_for_an_empty_argument_list() {
        assert!(should_launch(&[]));

        for args in [
            vec!["p"],
            vec![""],
            vec!["-p"],
            vec!["-a"],
            vec!["-i"],
            vec!["-h"],
            vec!["-V"],
            vec!["-m"],
            vec!["--"],
            vec!["-s"],
            vec!["--no-simplificar"],
            vec!["-s", "p"],
        ] {
            assert!(!should_launch(&arguments(&args)), "arguments {args:?}");
        }
    }

    #[test]
    fn the_startup_error_is_the_documented_spanish_message() {
        assert_eq!(STARTUP_ERROR, "no se pudo iniciar la interfaz gráfica");
    }

    // --- Default state -----------------------------------------------------

    #[test]
    fn the_default_app_has_an_empty_input_and_the_documented_controls() {
        let app = PlogicApp::default();

        assert_eq!(app.input, "");
        assert!(app.simplify);
        assert!(app.show_table);
        assert!(!app.show_process);
        assert!(app.outcome.is_none());
        assert!(app.cursor.is_none());
    }

    // --- Evaluate with the defaults ---------------------------------------

    #[test]
    fn evaluating_with_the_defaults_keeps_one_simplified_evaluation() {
        let app = app_with("p -> q");
        let evaluation = one_evaluation(&app);

        assert_eq!(evaluation.table.simplified.as_deref(), Some("q ∨ ¬p"));
        assert_eq!(evaluation.table.label, "q ∨ ¬p");
        let simplification = evaluation
            .simplification
            .as_ref()
            .expect("the default run should simplify");
        assert_eq!(simplification.steps.len(), 1);
        assert_eq!(simplification.steps[0].rule, Rule::Conditional);
        assert_eq!(simplification.steps[0].rule.label(), "13a. Condicional");
        assert_eq!(
            InfixPrinter::print(&simplification.steps[0].expression),
            "q ∨ ¬p"
        );
        assert_eq!(evaluation.table.classify(), Classification::Contingency);
    }

    #[test]
    fn the_formula_title_of_a_simplified_table_is_simplificada() {
        assert_eq!(
            formula_title(&displayed_table("p -> q", true)),
            "Simplificada"
        );
    }

    #[test]
    fn the_formula_title_of_an_original_table_is_original() {
        assert_eq!(formula_title(&displayed_table("p -> q", false)), "Original");
    }

    // --- Evaluate with simplify off ---------------------------------------

    #[test]
    fn evaluating_without_simplifying_tabulates_the_original_formula() {
        let mut app = PlogicApp {
            input: "p -> q".to_string(),
            simplify: false,
            ..Default::default()
        };
        app.evaluate();

        let evaluation = one_evaluation(&app);

        assert_eq!(evaluation.table.simplified, None);
        assert_eq!(evaluation.table.label, "p → q");
        assert_eq!(evaluation.table.variables, ["p", "q"]);
        assert!(evaluation.simplification.is_none());
    }

    // --- Result lifetime ---------------------------------------------------

    #[test]
    fn editing_the_input_does_not_replace_the_stored_outcome() {
        let mut app = app_with("p -> q");
        let before = evaluations(&app).to_vec();

        app.input.push_str(" & r");

        assert_eq!(evaluations(&app), before.as_slice());
    }

    #[test]
    fn inserting_at_the_cursor_does_not_replace_the_stored_outcome() {
        let ctx = egui::Context::default();
        let mut app = app_with("p -> q");
        let before = evaluations(&app).to_vec();
        app.cursor = Some(CCursorRange::one(CCursor::end_of_str(&app.input)));

        app.insert_at_cursor(&ctx, " & r");

        assert_eq!(app.input, "p -> q & r");
        assert_eq!(evaluations(&app), before.as_slice());
    }

    #[test]
    fn clearing_the_input_and_cursor_does_not_replace_the_stored_outcome() {
        let mut app = app_with("p -> q");
        let before = evaluations(&app).to_vec();
        app.cursor = Some(CCursorRange::one(CCursor::new(2)));

        // `Limpiar` is UI-only: it clears these two fields and nothing else.
        app.input.clear();
        app.cursor = None;

        assert_eq!(app.input, "");
        assert!(app.cursor.is_none());
        assert_eq!(evaluations(&app), before.as_slice());
    }

    #[test]
    fn changing_simplify_does_not_replace_the_stored_outcome() {
        let mut app = app_with("p -> q");
        let before = evaluations(&app).to_vec();

        app.simplify = false;

        assert_eq!(evaluations(&app), before.as_slice());
    }

    #[test]
    fn a_second_evaluate_replaces_the_first_outcome() {
        let mut app = app_with("p -> q");

        app.input = "p & q".to_string();
        app.evaluate();

        let evaluations = evaluations(&app);
        assert_eq!(evaluations.len(), 1);
        assert_eq!(evaluations[0].table.label, "p ∧ q");
    }

    // --- Errors ------------------------------------------------------------

    #[test]
    fn evaluating_a_blank_input_reports_the_missing_proposition() {
        for source in ["", " ", "  \n\t "] {
            let mut app = PlogicApp {
                input: source.to_string(),
                ..Default::default()
            };

            app.evaluate();

            assert_eq!(errors(&app).len(), 1, "source {source:?}");
            assert_eq!(
                errors(&app)[0],
                "no se proporcionó ninguna proposición",
                "source {source:?}"
            );
            assert_eq!(app.input, source, "the field must not be cleared");
        }
    }

    #[test]
    fn a_parse_error_is_reported_as_a_line_per_error() {
        let app = app_with("p &");

        let errors = errors(&app);
        assert!(!errors.is_empty());
        assert!(errors.iter().all(|line| !line.trim().is_empty()));
        assert!(
            errors
                .iter()
                .any(|line| line.contains("se esperaba una expresión")),
            "reports {errors:?}"
        );
    }

    #[test]
    fn a_later_valid_evaluate_replaces_a_parse_error() {
        let mut app = app_with("p &");

        app.input = "p".to_string();
        app.evaluate();

        let evaluations = evaluations(&app);
        assert_eq!(evaluations.len(), 1);
        assert_eq!(evaluations[0].table.label, "p");
    }

    // --- next_variable -----------------------------------------------------

    #[test]
    fn next_variable_starts_at_p_for_an_empty_input() {
        assert_eq!(next_variable(""), "p");
    }

    #[test]
    fn next_variable_skips_the_letters_already_used() {
        assert_eq!(next_variable("p"), "q");
        assert_eq!(next_variable("p & q"), "r");
    }

    #[test]
    fn next_variable_wraps_to_p_when_every_letter_is_used() {
        let used: Vec<String> = ('p'..='z').map(|letter| letter.to_string()).collect();

        assert_eq!(next_variable(&used.join(" & ")), "p");
    }

    #[test]
    fn next_variable_falls_back_to_p_when_the_formula_does_not_scan() {
        assert_eq!(next_variable("%"), "p");
    }

    // --- byte_index --------------------------------------------------------

    #[test]
    fn byte_index_maps_character_indices_to_byte_offsets_for_ascii() {
        let text = "p & q";

        assert_eq!(byte_index(text, 0), 0);
        assert_eq!(byte_index(text, 2), 2);
        assert_eq!(byte_index(text, 5), 5);
        assert_eq!(byte_index(text, text.chars().count()), text.len());
    }

    #[test]
    fn byte_index_counts_the_bytes_of_a_multi_byte_symbol() {
        let text = "p ∧ q";

        assert_eq!(byte_index(text, 0), 0);
        assert_eq!(byte_index(text, 2), 2);
        assert_eq!(byte_index(text, 3), 5);
        assert_eq!(byte_index(text, 4), 6);
        assert_eq!(byte_index(text, text.chars().count()), text.len());
        assert_eq!(&text[..byte_index(text, 3)], "p ∧");
    }

    #[test]
    fn byte_index_of_a_position_past_the_end_is_the_text_length() {
        assert_eq!(byte_index("p ∧ q", 99), "p ∧ q".len());
    }

    // --- insert_at_cursor --------------------------------------------------

    #[test]
    fn insert_at_cursor_replaces_a_selection_and_leaves_the_cursor_after_it() {
        let ctx = egui::Context::default();
        let mut app = PlogicApp {
            input: "p -> q".to_string(),
            cursor: Some(CCursorRange::two(CCursor::new(2), CCursor::new(4))),
            ..Default::default()
        };

        app.insert_at_cursor(&ctx, "∧");

        assert_eq!(app.input, "p ∧ q");
        assert_eq!(app.cursor, Some(CCursorRange::one(CCursor::new(3))));
    }

    #[test]
    fn insert_at_cursor_appends_when_the_cursor_is_unknown() {
        let ctx = egui::Context::default();
        let mut app = PlogicApp {
            input: "p".to_string(),
            ..Default::default()
        };

        app.insert_at_cursor(&ctx, " ∧");

        assert_eq!(app.input, "p ∧");
        assert_eq!(app.cursor, Some(CCursorRange::one(CCursor::new(3))));
    }

    // --- value_cell --------------------------------------------------------

    #[test]
    fn value_cell_maps_true_to_a_green_v_and_false_to_a_red_f() {
        assert_eq!(value_cell(true), ("V", TRUE_COLOR));
        assert_eq!(value_cell(false), ("F", FALSE_COLOR));
    }

    // --- classification_display -------------------------------------------

    #[test]
    fn classification_display_calls_a_tautology_green() {
        assert_eq!(
            classification_display(&displayed_table("p | !p", true)),
            ("Esta proposición es una tautología.", TRUE_COLOR)
        );
    }

    #[test]
    fn classification_display_calls_a_contradiction_red() {
        assert_eq!(
            classification_display(&displayed_table("p & !p", true)),
            ("Esta proposición es una contradicción.", FALSE_COLOR)
        );
    }

    #[test]
    fn classification_display_calls_a_contingency_muted() {
        assert_eq!(
            classification_display(&displayed_table("p", true)),
            ("Esta proposición es una contingencia.", MUTED)
        );
    }

    // --- Connective help ---------------------------------------------------

    #[test]
    fn the_connective_help_lists_the_seven_symbols_with_their_spellings() {
        let entries: Vec<HelpTriplet> = CONNECTIVE_HELP
            .iter()
            .flat_map(|row| std::iter::once(row.left).chain(row.right))
            .collect();
        let expected = [
            ("¬", "negación", "!, no"),
            ("∧", "conjunción", "&, y"),
            ("∨", "disyunción", "|, o"),
            ("→", "implicación", "->, entonces"),
            ("↔", "bicondicional", "<->, sisolosi"),
            ("V", "verdadero", "1"),
            ("F", "falso", "0"),
        ];

        assert_eq!(entries.as_slice(), expected.as_slice());
    }

    #[test]
    #[allow(clippy::assertions_on_constants)] // both sizes are design constants
    fn the_help_font_is_smaller_than_the_body_text() {
        assert!(HELP_FONT < 15.0, "help font is {HELP_FONT}");
    }

    // --- Table data --------------------------------------------------------

    #[test]
    fn the_implication_table_uses_the_simplified_columns_and_four_rows() {
        let table = displayed_table("p -> q", true);

        assert_eq!(table.variables, ["q", "p"]);
        assert_eq!(table.rows.len(), 4);
    }

    #[test]
    fn a_one_variable_table_has_one_column_and_two_rows() {
        let table = displayed_table("p", true);

        assert_eq!(table.variables, ["p"]);
        assert_eq!(table.rows.len(), 2);
        assert!(table.rows.iter().all(|row| row.values.len() == 1));
    }
}
