//! The graphical window.
//!
//! Bare `plogic` (no arguments at all) opens this window; every argument keeps
//! the command-line interface (`design/gui.md`, ADR-048). The GUI is another
//! impure UI component: it drives the same [`Orchestrator`] as the CLI and
//! builds its own components from the returned data. The pipeline stays pure.
//!
//! Layout: one centered column of fixed maximum width in a single vertical
//! scroll area. From top to bottom it holds the header (title and collapsible
//! help), the input section (formula, symbol toolbar, options and the
//! `Evaluar` button) and the result section. The result components flow in the
//! main window instead of a separate result pane, so the whole window scrolls
//! as one.

use std::collections::HashSet;
use std::ffi::OsString;

use eframe::egui;
use egui::text::{CCursor, CCursorRange};

use crate::args::UsageError;
use crate::orchestrator::{Assessment, Orchestrator};
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
/// The faint background egui uses for striped widgets.
const RESULT_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(12, 12, 15);
/// The background of the table header.
const HEADER_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(24, 24, 30);
/// The background of every other table row.
const ROW_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(20, 20, 25);
/// The fill of the primary `Evaluar` button.
const ACCENT: egui::Color32 = egui::Color32::from_rgb(58, 74, 140);
/// The fill of the primary button while hovered.
const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(72, 90, 164);
/// The normal text color.
const TEXT: egui::Color32 = egui::Color32::from_rgb(232, 232, 236);
/// Secondary text (hints and result labels).
const MUTED: egui::Color32 = egui::Color32::from_rgb(148, 148, 158);
/// The border of the framed controls and the separators.
const BORDER: egui::Color32 = egui::Color32::from_rgb(56, 56, 64);
/// The background of a hovered widget.
const HOVER: egui::Color32 = egui::Color32::from_rgb(28, 28, 34);
/// The selection background.
const SELECTION: egui::Color32 = egui::Color32::from_rgb(60, 60, 72);
/// The color of an error line.
const ERROR: egui::Color32 = egui::Color32::from_rgb(255, 120, 120);
/// The color of a true result and of a tautology.
const TRUE_COLOR: egui::Color32 = egui::Color32::from_rgb(120, 220, 150);
/// The color of a false result and of a contradiction.
const FALSE_COLOR: egui::Color32 = egui::Color32::from_rgb(240, 120, 120);

/// The widest the central column gets; wider windows add side margins.
const MAX_WIDTH: f32 = 760.0;
/// The vertical space between the three main sections.
const SECTION_GAP: f32 = 14.0;
/// The vertical space between a label and what it describes.
const LABEL_GAP: f32 = 6.0;
/// The height of the formula field.
const INPUT_HEIGHT: f32 = 40.0;
/// The height of the `Evaluar` button.
const EVALUATE_HEIGHT: f32 = 40.0;

/// The help is deliberately a little smaller than the body text.
const HELP_FONT: f32 = 12.5;
/// The column headers of the connector list.
const HELP_SMALL: f32 = 11.0;
/// The help headings.
const HELP_HEADING: f32 = 13.5;
/// The title of a section (`Proposición`, `Resultado`).
const SECTION_TITLE: f32 = 16.0;
/// The table cells and their header.
const TABLE_FONT: f32 = 13.0;
/// The small label above each result component.
const LABEL_FONT: f32 = 13.0;
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

/// The rows of the help list: formal symbol, Spanish name, other spellings.
const CONNECTIVE_HELP: &[(&str, &str, &str)] = &[
    ("¬", "negación", "!, no"),
    ("∧", "conjunción", "&, y"),
    ("∨", "disyunción", "|, o"),
    ("→", "implicación", "->, entonces"),
    ("↔", "bicondicional", "<->, sisolosi"),
    ("V", "verdadero", "1"),
    ("F", "falso", "0"),
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
            .with_inner_size([860.0, 880.0])
            .with_min_inner_size([620.0, 620.0])
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

/// The options captured when `Evaluar` was pressed. They are fixed for the
/// stored result; changing a checkbox only affects the next evaluation.
struct View {
    assessments: Vec<Assessment>,
    show_table: bool,
    show_process: bool,
}

/// What the result section shows: either the last evaluation or the errors
/// that replaced it. A new evaluation overwrites the whole value.
enum Outcome {
    Evaluations(View),
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

    /// The whole window, from top to bottom (`design/gui.md`): header, input
    /// section, result section.
    fn contents(&mut self, ui: &mut egui::Ui) {
        header(ui);
        ui.add_space(SECTION_GAP);

        let evaluate = self.input_section(ui);
        if evaluate {
            self.evaluate();
        }

        ui.add_space(SECTION_GAP);
        ui.separator();
        ui.add_space(SECTION_GAP);

        self.result_section(ui);
    }

    /// The formula field, the symbol toolbar, the options and the `Evaluar`
    /// button, stacked in that order. Returns whether the user asked to
    /// evaluate, by the button or by pressing Enter in the field.
    fn input_section(&mut self, ui: &mut egui::Ui) -> bool {
        ui.horizontal(|ui| {
            section_title(ui, "Proposición");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new("Enter para evaluar")
                        .monospace()
                        .size(HELP_SMALL)
                        .color(MUTED),
                );
            });
        });
        ui.add_space(LABEL_GAP);

        let output = egui::TextEdit::singleline(&mut self.input)
            .id(input_id())
            .hint_text("p | q & s -> t")
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .min_size(egui::vec2(0.0, INPUT_HEIGHT))
            .margin(egui::Margin::symmetric(14, 10))
            .background_color(INPUT_BACKGROUND)
            .text_color(TEXT)
            .show(ui);
        self.remember_cursor(output.cursor_range);
        let enter =
            output.response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));

        ui.add_space(LABEL_GAP);
        self.toolbar(ui);

        ui.add_space(SECTION_GAP);
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.simplify, "Simplificar");
            ui.checkbox(&mut self.show_table, "Mostrar tabla");
            ui.checkbox(&mut self.show_process, "Mostrar proceso");
        });

        ui.add_space(SECTION_GAP);
        let button = egui::Button::new(
            egui::RichText::new("Evaluar")
                .monospace()
                .size(17.0)
                .strong(),
        )
        .fill(ACCENT);
        let clicked = ui
            .scope(|ui| {
                ui.visuals_mut().widgets.hovered.weak_bg_fill = ACCENT_HOVER;
                ui.add_sized([ui.available_width(), EVALUATE_HEIGHT], button)
                    .clicked()
            })
            .inner;

        clicked || enter
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

            ui.separator();

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

    /// Keeps the caret of the formula field across frames, including the frame
    /// in which the field loses focus to a toolbar button. A frame without a
    /// caret range (unfocused field) must not erase the last known one.
    fn remember_cursor(&mut self, cursor: Option<CCursorRange>) {
        self.cursor = cursor.or(self.cursor);
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
        let show_table = self.show_table;
        let show_process = self.show_process;
        let max_variables = show_table.then_some(10);
        match Orchestrator::assess(self.input.clone(), self.simplify, max_variables) {
            Ok(assessments) if assessments.is_empty() => {
                self.outcome = Some(Outcome::Errors(vec![no_proposition()]));
            }
            Ok(assessments) => {
                self.outcome = Some(Outcome::Evaluations(View {
                    assessments,
                    show_table,
                    show_process,
                }));
            }
            Err(errors) => {
                self.outcome = Some(Outcome::Errors(
                    errors.into_iter().map(Orchestrator::report).collect(),
                ));
            }
        }
    }

    /// The result section. Its components flow directly in the main window;
    /// the output is centered; each component is a small label followed by its
    /// content. The previous result, including the options it was evaluated
    /// with, stays until `Evaluar` replaces it.
    fn result_section(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| section_title(ui, "Resultado"));
        ui.add_space(LABEL_GAP);

        ui.vertical_centered(|ui| match &self.outcome {
            None => {
                ui.add_space(SECTION_GAP);
                ui.label(
                    egui::RichText::new("El resultado aparecerá aquí.")
                        .monospace()
                        .size(12.0)
                        .color(MUTED),
                );
            }
            Some(Outcome::Errors(errors)) => errors_component(ui, errors),
            Some(Outcome::Evaluations(view)) => {
                for (index, assessment) in view.assessments.iter().enumerate() {
                    if index > 0 {
                        ui.add_space(SECTION_GAP);
                        ui.separator();
                        ui.add_space(SECTION_GAP);
                    }
                    self.evaluation(ui, assessment, view.show_table, view.show_process);
                }
                ui.add_space(SECTION_GAP);
            }
        });
    }

    /// One assessment as its components: the formula, the optional process
    /// list, the optional table and the classification only when a table was
    /// processed.
    fn evaluation(
        &self,
        ui: &mut egui::Ui,
        assessment: &Assessment,
        show_table: bool,
        show_process: bool,
    ) {
        component(ui, formula_title(assessment), |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(&assessment.formula)
                        .monospace()
                        .size(20.0)
                        .color(TEXT),
                )
                .selectable(true)
                .wrap()
                .halign(egui::Align::Center),
            );
        });

        if show_process
            && let Some(simplification) = &assessment.simplification
            && !simplification.steps.is_empty()
        {
            ui.add_space(SECTION_GAP);
            component(ui, "Proceso", |ui| {
                process_list(ui, &simplification.original, &simplification.steps);
            });
        }

        if show_table {
            ui.add_space(SECTION_GAP);
            if let Some(table) = &assessment.table {
                component(ui, "Tabla de verdad", |ui| {
                    table_component(ui, table);
                });
                ui.add_space(SECTION_GAP);
                component(ui, "Clasificación", |ui| {
                    classification_component(ui, table);
                });
            } else {
                component(ui, "Tabla de verdad", |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "La tabla no se procesa: {} variables (el límite es 10).",
                            assessment.variables
                        ))
                        .monospace()
                        .size(13.0)
                        .color(MUTED),
                    );
                });
            }
        }
    }
}

impl eframe::App for PlogicApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BACKGROUND)
                    .inner_margin(egui::Margin::symmetric(24, 20)),
            )
            .show(ui, |ui| {
                // One vertical scroll area for the whole window. Inside it,
                // the content stays in a centered column of bounded width.
                egui::ScrollArea::vertical()
                    .id_salt("plogic-window")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let width = ui.available_width().min(MAX_WIDTH);
                        let side = ((ui.available_width() - width) * 0.5).max(0.0);
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

/// The title and the collapsible help. The help starts closed so it does not
/// push the controls down; open it with the `Ayuda` header.
fn header(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new("plogic")
                .monospace()
                .size(34.0)
                .strong(),
        );
        ui.label(
            egui::RichText::new(
                "Evalúa proposiciones de lógica proposicional y construye su tabla de verdad.",
            )
            .monospace()
            .size(HELP_FONT)
            .color(MUTED),
        );
    });
    ui.add_space(LABEL_GAP);
    egui::CollapsingHeader::new(
        egui::RichText::new("Ayuda")
            .monospace()
            .strong()
            .size(HELP_HEADING),
    )
    .default_open(false)
    .show(ui, |ui| {
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
    });
}

/// The heading of a section.
fn section_title(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .monospace()
            .strong()
            .size(SECTION_TITLE),
    );
}

/// The connector and literal help: one set of three columns with the formal symbol,
/// the Spanish name and the other accepted spellings.
fn connectors_help(ui: &mut egui::Ui) {
    egui::Grid::new("plogic-connectors")
        .num_columns(3)
        .spacing([18.0, 2.0])
        .show(ui, |ui| {
            help_headers(ui);
            ui.end_row();

            for &(symbol, name, spellings) in CONNECTIVE_HELP {
                help_triplet(ui, symbol, name, spellings);
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

/// A result component without a box: a small muted label and its body.
fn component(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.label(
        egui::RichText::new(title)
            .monospace()
            .strong()
            .size(LABEL_FONT)
            .color(MUTED),
    );
    ui.add_space(LABEL_GAP);
    body(ui);
}

/// One row per applied law, as a numbered list.
fn process_list(ui: &mut egui::Ui, original: &str, steps: &[Step]) {
    fn add_list_item(ui: &mut egui::Ui, index: usize, rule: &str, expression: &str) {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(format!("{}. {}:", index, rule))
                    .monospace()
                    .size(14.0),
            );
            ui.label(egui::RichText::new(expression).monospace().size(14.0));
        });
    }
    add_list_item(ui, 0, "Original", original);
    for (index, step) in steps.iter().enumerate() {
        add_list_item(
            ui,
            index + 1,
            step.rule.label(),
            &step.expression.infix_str(),
        );
    }
}

/// The errors: a red label and one selectable red line per report.
fn errors_component(ui: &mut egui::Ui, errors: &[String]) {
    ui.label(
        egui::RichText::new("Error")
            .monospace()
            .strong()
            .size(LABEL_FONT)
            .color(ERROR),
    );
    ui.add_space(LABEL_GAP);
    for error in errors {
        ui.add(
            egui::Label::new(egui::RichText::new(error.as_str()).monospace().color(ERROR))
                .selectable(true),
        );
    }
}

/// A real truth table: a fixed header, a variable column per variable, a
/// separator and the result column. The whole table scrolls horizontally when
/// the columns do not fit; vertically it flows with the rest of the result.
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
    let scroll_width = table_width.min(ui.available_width());
    let row_height = (TABLE_FONT + 10.0).max(ui.text_style_height(&egui::TextStyle::Monospace));

    ui.horizontal(|ui| {
        let side = ((ui.available_width() - scroll_width) * 0.5).max(0.0);
        ui.add_space(side);
        egui::ScrollArea::horizontal()
            .id_salt("plogic-table-horizontal")
            .max_width(scroll_width)
            .auto_shrink([true, true])
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
                                        table_cell(
                                            ui,
                                            text,
                                            result_width,
                                            row_height,
                                            color,
                                            false,
                                        );
                                    });
                                });
                            });
                        }
                    });
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

fn formula_title(assessment: &Assessment) -> &'static str {
    if assessment.simplification.is_some() {
        "Simplificada"
    } else {
        "Original"
    }
}

/// The classification sentence, colored by the verdict.
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

    fn view_of(app: &PlogicApp) -> &View {
        match &app.outcome {
            Some(Outcome::Evaluations(view)) => view,
            Some(Outcome::Errors(errors)) => {
                panic!("expected an evaluation, got errors: {errors:?}")
            }
            None => panic!("expected an evaluation, got no outcome"),
        }
    }

    fn errors(app: &PlogicApp) -> &[String] {
        match &app.outcome {
            Some(Outcome::Errors(errors)) => errors,
            Some(Outcome::Evaluations(view)) => panic!(
                "expected errors, got {} assessments",
                view.assessments.len()
            ),
            None => panic!("expected errors, got no outcome"),
        }
    }

    fn one_assessment(app: &PlogicApp) -> &Assessment {
        let assessments = &view_of(app).assessments;
        assert_eq!(assessments.len(), 1);
        &assessments[0]
    }

    /// The stored result as plain data, so a test can compare it before and
    /// after an action that must not replace it.
    fn stored(app: &PlogicApp) -> (Vec<Assessment>, bool, bool) {
        let view = view_of(app);
        (view.assessments.clone(), view.show_table, view.show_process)
    }

    fn app_with(source: &str) -> PlogicApp {
        let mut app = PlogicApp {
            input: source.to_string(),
            ..Default::default()
        };
        app.evaluate();
        app
    }

    fn app_with_options(
        source: &str,
        simplify: bool,
        show_table: bool,
        show_process: bool,
    ) -> PlogicApp {
        let mut app = PlogicApp {
            input: source.to_string(),
            simplify,
            show_table,
            show_process,
            ..Default::default()
        };
        app.evaluate();
        app
    }

    /// One assessment of `source`, tabulated when it has fewer than ten
    /// variables, as the GUI asks for it.
    fn assessed(source: &str, simplify: bool) -> Assessment {
        let mut assessments = Orchestrator::assess(source.to_string(), simplify, Some(10))
            .unwrap_or_else(|errors| panic!("source {source:?} failed: {errors:?}"));
        assert_eq!(assessments.len(), 1, "source {source:?}");
        assessments.remove(0)
    }

    fn table_of(source: &str) -> TruthTable {
        assessed(source, true)
            .table
            .unwrap_or_else(|| panic!("source {source:?} should have a table"))
    }

    /// `p1 | p2 | ... | pN`: a chain with exactly `count` distinct variables.
    fn variables_or_chain(count: usize) -> String {
        let names: Vec<String> = (1..=count).map(|index| format!("p{index}")).collect();
        names.join(" | ")
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

        assert!(view_of(&app).show_table);
        assert!(!view_of(&app).show_process);
        let assessment = one_assessment(&app);
        assert_eq!(assessment.formula, "q ∨ ¬p");
        assert_eq!(assessment.variables, 2);
        let simplification = assessment
            .simplification
            .as_ref()
            .expect("the default run should simplify");
        assert_eq!(simplification.steps.len(), 1);
        assert_eq!(simplification.steps[0].rule, Rule::Conditional);
        assert_eq!(simplification.steps[0].rule.label(), "13a. Condicional");
        let table = assessment
            .table
            .as_ref()
            .expect("two variables fit below ten");
        assert_eq!(table.label, "q ∨ ¬p");
        assert_eq!(table.classify(), Classification::Contingency);
    }

    // --- Table off ---------------------------------------------------------

    #[test]
    fn evaluating_with_the_table_off_builds_no_table_and_keeps_the_flag() {
        let mut app = app_with_options("p -> q", true, false, false);

        assert!(!view_of(&app).show_table);
        assert!(
            view_of(&app).assessments.iter().all(|a| a.table.is_none()),
            "the table must not be built"
        );

        let before = stored(&app);
        app.show_table = true;

        assert_eq!(
            stored(&app),
            before,
            "turning the table on is not an evaluation"
        );
        assert!(!view_of(&app).show_table);

        app.evaluate();

        assert!(view_of(&app).show_table);
        assert!(one_assessment(&app).table.is_some());
    }

    // --- Table policy ------------------------------------------------------

    #[test]
    fn evaluating_a_nine_variable_proposition_builds_its_table() {
        let app = app_with(&variables_or_chain(9));
        let assessment = one_assessment(&app);

        assert_eq!(assessment.variables, 9);
        assert!(assessment.table.is_some());
    }

    #[test]
    fn evaluating_a_ten_variable_proposition_skips_the_table() {
        let app = app_with(&variables_or_chain(10));
        let assessment = one_assessment(&app);

        assert_eq!(assessment.variables, 10);
        assert_eq!(assessment.table, None);
        assert!(!assessment.formula.is_empty());
        assert!(assessment.simplification.is_some());
    }

    // --- Process snapshot --------------------------------------------------

    #[test]
    fn the_process_control_is_snapshotted_at_evaluar() {
        let mut app = app_with_options("p -> q", true, true, false);

        assert!(!view_of(&app).show_process);

        let before = stored(&app);
        app.show_process = true;

        assert_eq!(
            stored(&app),
            before,
            "turning the process on is not an evaluation"
        );
        assert!(!view_of(&app).show_process);

        app.evaluate();

        assert!(view_of(&app).show_process);
    }

    // --- Simplify snapshot -------------------------------------------------

    #[test]
    fn the_simplify_control_is_snapshotted_at_evaluar() {
        let mut app = app_with("p -> q");

        assert!(one_assessment(&app).simplification.is_some());

        let before = stored(&app);
        app.simplify = false;

        assert_eq!(
            stored(&app),
            before,
            "unchecking Simplificar is not an evaluation"
        );

        app.evaluate();

        assert_eq!(one_assessment(&app).simplification, None);
        assert_eq!(one_assessment(&app).formula, "p → q");
    }

    // --- Result lifetime ---------------------------------------------------

    #[test]
    fn editing_the_input_does_not_replace_the_stored_outcome() {
        let mut app = app_with("p -> q");
        let before = stored(&app);

        app.input.push_str(" & r");

        assert_eq!(stored(&app), before);
    }

    #[test]
    fn inserting_at_the_cursor_does_not_replace_the_stored_outcome() {
        let ctx = egui::Context::default();
        let mut app = app_with("p -> q");
        let before = stored(&app);
        app.cursor = Some(CCursorRange::one(CCursor::end_of_str(&app.input)));

        app.insert_at_cursor(&ctx, " & r");

        assert_eq!(app.input, "p -> q & r");
        assert_eq!(stored(&app), before);
    }

    #[test]
    fn changing_the_controls_does_not_replace_the_stored_outcome() {
        let mut app = app_with("p -> q");
        let before = stored(&app);

        app.simplify = false;
        app.show_table = false;
        app.show_process = true;

        assert_eq!(stored(&app), before);
    }

    #[test]
    fn clearing_the_input_and_cursor_keeps_the_stored_outcome() {
        let mut app = app_with("p -> q");
        let before = stored(&app);
        app.cursor = Some(CCursorRange::one(CCursor::new(2)));

        // `Limpiar` is UI-only: it clears these two fields and nothing else.
        app.input.clear();
        app.cursor = None;

        assert_eq!(app.input, "");
        assert!(app.cursor.is_none());
        assert_eq!(stored(&app), before);
    }

    #[test]
    fn a_second_evaluate_replaces_the_whole_outcome() {
        let mut app = app_with("p -> q");

        app.input = "p & q".to_string();
        app.evaluate();

        let assessment = one_assessment(&app);
        assert_eq!(assessment.formula, "p ∧ q");
        assert_eq!(assessment.variables, 2);
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

        let assessment = one_assessment(&app);
        assert_eq!(assessment.formula, "p");
        assert_eq!(
            assessment.table.as_ref().map(|table| table.label.as_str()),
            Some("p")
        );
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
    fn next_variable_skips_y_for_conjunction() {
        let used: Vec<String> = ('p'..='x').map(|letter| letter.to_string()).collect();

        assert_eq!(next_variable(&used.join(" & ")), "z");
    }

    #[test]
    fn next_variable_falls_back_to_p_when_every_usable_letter_is_used() {
        let used: Vec<String> = ('p'..='z')
            .filter(|letter| *letter != 'y')
            .map(|letter| letter.to_string())
            .collect();

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

    // --- remember_cursor ---------------------------------------------------

    #[test]
    fn remember_cursor_keeps_the_last_known_caret_when_the_field_loses_focus() {
        let mut app = PlogicApp {
            input: "pq".to_string(),
            cursor: Some(CCursorRange::one(CCursor::new(1))),
            ..Default::default()
        };

        app.remember_cursor(None);

        assert_eq!(app.cursor, Some(CCursorRange::one(CCursor::new(1))));
    }

    #[test]
    fn remember_cursor_replaces_the_caret_when_the_field_reports_one() {
        let mut app = PlogicApp {
            cursor: Some(CCursorRange::one(CCursor::new(1))),
            ..Default::default()
        };

        app.remember_cursor(Some(CCursorRange::one(CCursor::new(2))));

        assert_eq!(app.cursor, Some(CCursorRange::one(CCursor::new(2))));
    }

    #[test]
    fn remember_cursor_without_a_caret_keeps_an_unknown_caret_unknown() {
        let mut app = PlogicApp::default();

        app.remember_cursor(None);

        assert!(app.cursor.is_none());
    }

    #[test]
    fn a_toolbar_insert_after_focus_loss_uses_the_remembered_caret() {
        let ctx = egui::Context::default();
        let mut app = PlogicApp {
            input: "pq".to_string(),
            cursor: Some(CCursorRange::one(CCursor::new(1))),
            ..Default::default()
        };

        app.remember_cursor(None);
        app.insert_at_cursor(&ctx, "∧");

        assert_eq!(app.input, "p∧q");
        assert_eq!(app.cursor, Some(CCursorRange::one(CCursor::new(2))));
    }

    // --- value_cell --------------------------------------------------------

    #[test]
    fn value_cell_maps_true_to_a_green_v_and_false_to_a_red_f() {
        assert_eq!(value_cell(true), ("V", TRUE_COLOR));
        assert_eq!(value_cell(false), ("F", FALSE_COLOR));
    }

    // --- formula_title -----------------------------------------------------

    #[test]
    fn the_formula_title_is_simplificada_when_the_simplifier_ran() {
        assert_eq!(formula_title(&assessed("p -> q", true)), "Simplificada");
    }

    #[test]
    fn the_formula_title_is_original_without_a_simplification() {
        assert_eq!(formula_title(&assessed("p -> q", false)), "Original");
    }

    // --- classification_display -------------------------------------------

    #[test]
    fn classification_display_calls_a_tautology_green() {
        assert_eq!(
            classification_display(&table_of("p | !p")),
            ("Esta proposición es una tautología.", TRUE_COLOR)
        );
    }

    #[test]
    fn classification_display_calls_a_contradiction_red() {
        assert_eq!(
            classification_display(&table_of("p & !p")),
            ("Esta proposición es una contradicción.", FALSE_COLOR)
        );
    }

    #[test]
    fn classification_display_calls_a_contingency_muted() {
        assert_eq!(
            classification_display(&table_of("p -> q")),
            ("Esta proposición es una contingencia.", MUTED)
        );
    }

    // --- Connective help ---------------------------------------------------

    #[test]
    fn the_connective_help_lists_the_seven_symbols_with_their_spellings() {
        let expected = [
            ("¬", "negación", "!, no"),
            ("∧", "conjunción", "&, y"),
            ("∨", "disyunción", "|, o"),
            ("→", "implicación", "->, entonces"),
            ("↔", "bicondicional", "<->, sisolosi"),
            ("V", "verdadero", "1"),
            ("F", "falso", "0"),
        ];

        assert_eq!(CONNECTIVE_HELP.len(), 7);
        assert_eq!(CONNECTIVE_HELP, expected);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)] // both sizes are design constants
    fn the_help_font_is_smaller_than_the_body_text() {
        assert!(HELP_FONT < 15.0, "help font is {HELP_FONT}");
    }

    // --- Table data --------------------------------------------------------

    #[test]
    fn the_implication_table_uses_the_simplified_columns_and_four_rows() {
        let assessment = assessed("p -> q", true);
        let table = assessment.table.expect("two variables fit below ten");

        assert_eq!(table.variables, ["q", "p"]);
        assert_eq!(table.rows.len(), 4);
    }

    #[test]
    fn a_one_variable_table_has_one_column_and_two_rows() {
        let assessment = assessed("p", true);
        let table = assessment.table.expect("one variable fits below ten");

        assert_eq!(table.variables, ["p"]);
        assert_eq!(table.rows.len(), 2);
        assert!(table.rows.iter().all(|row| row.values.len() == 1));
    }
}
