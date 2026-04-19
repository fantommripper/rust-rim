use egui::{Color32, Rect, RichText, Sense, Ui, Vec2};
use egui_extras::{Column, TableBuilder};

use crate::app::{source_color, source_label, theme, DragPayload, MoveRequest};
use crate::mod_data::{ModEntry, ModSource};

const ROW_HEIGHT: f32 = 22.0;

pub struct ModList<'a> {
    mods:      &'a mut Vec<ModEntry>,
    indices:   &'a [usize],
    selected:  &'a mut Option<usize>,
    is_active: bool,
}

impl<'a> ModList<'a> {
    pub fn new(
        mods:      &'a mut Vec<ModEntry>,
        indices:   &'a [usize],
        selected:  &'a mut Option<usize>,
        is_active: bool,
    ) -> Self {
        Self { mods, indices, selected, is_active }
    }

    pub fn show(self, ui: &mut Ui) -> Option<MoveRequest> {
        let ctx = ui.ctx().clone();

        let active_ids: std::collections::HashSet<String> = self.mods.iter()
            .filter(|m| m.is_active)
            .map(|m| m.package_id.clone())
            .collect();

        let available_height = ui.available_height();

        let panel_key      = if self.is_active { "active_list"    } else { "inactive_list"    };
        let hover_key      = egui::Id::new(if self.is_active { "active_hover"  } else { "inactive_hover"  });
        let drop_key       = egui::Id::new(if self.is_active { "active_drop"   } else { "inactive_drop"   });
        let is_dragging    = egui::DragAndDrop::has_any_payload(&ctx);
        let prev_hovered:  Option<usize> = ctx.data(|d| d.get_temp(hover_key));
        let prev_drop_row: Option<usize> = ctx.data(|d| d.get_temp(drop_key));

        let result = ui.push_id(panel_key, |ui| {
            let mut move_request: Option<MoveRequest> = None;
            let mut cur_hovered:  Option<usize> = None;
            let mut cur_drop_row: Option<usize> = None;

            let table = TableBuilder::new(ui)
                .striped(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(18.0))
                .column(Column::remainder().at_least(60.0).clip(true))
                .column(Column::exact(60.0))
                .column(Column::exact(20.0))
                .min_scrolled_height(available_height)
                .max_scroll_height(available_height)
                .auto_shrink([false, false])
                .sense(Sense::click_and_drag());

            table
                .header(20.0, |mut header| {
                    header.col(|ui| { ui.label(RichText::new("").size(10.0).color(theme::TEXT_MUTED)); });
                    header.col(|ui| { ui.label(RichText::new("НАЗВАНИЕ").size(10.0).color(theme::TEXT_MUTED).strong()); });
                    header.col(|ui| { ui.label(RichText::new("ВЕРСИЯ").size(10.0).color(theme::TEXT_MUTED).strong()); });
                    header.col(|ui| { ui.label(RichText::new("").size(10.0)); });
                })
                .body(|body| {
                    let num_rows = self.indices.len();

                    body.rows(ROW_HEIGHT, num_rows, |mut row| {
                        let row_pos  = row.index();
                        let orig_idx = self.indices[row_pos];
                        let is_selected      = *self.selected == Some(orig_idx);
                        let is_being_dragged = egui::DragAndDrop::payload::<DragPayload>(&ctx)
                            .map_or(false, |p| p.orig_idx == orig_idx);
                        let is_drop_target   = is_dragging && prev_drop_row == Some(row_pos) && !is_being_dragged;
                        let is_hovered       = !is_dragging && prev_hovered == Some(row_pos) && !is_selected;

                        let is_even    = row_pos % 2 == 0;
                        let base_color = if is_even { theme::BG_ROW_EVEN } else { theme::BG_ROW_ODD };

                        let has_missing_deps = self.mods[orig_idx].dependencies.iter()
                            .any(|d| !active_ids.contains(d));
                        let has_incompat = self.mods[orig_idx].is_active &&
                            self.mods[orig_idx].incompatible_with.iter()
                                .any(|ic| active_ids.contains(ic));

                        let row_bg = if is_being_dragged {
                            Color32::from_rgb(22, 24, 30)
                        } else if is_selected {
                            theme::BG_SELECTED
                        } else if is_hovered {
                            theme::BG_ROW_HOVER
                        } else {
                            base_color
                        };

                        let accent = if self.is_active { theme::HEADER_RIGHT } else { theme::HEADER_LEFT };

                        // Колонка: иконка источника
                        row.col(|ui| {
                            ui.push_id(orig_idx * 4, |ui| {
                                let rect = ui.max_rect();
                                ui.painter().rect_filled(rect, 0.0, row_bg);
                                if is_selected && !is_being_dragged {
                                    ui.painter().rect_filled(
                                        Rect::from_min_size(rect.left_top(), Vec2::new(2.0, ROW_HEIGHT)),
                                        0.0, accent,
                                    );
                                }
                                if is_drop_target {
                                    paint_drop_line(ui, rect);
                                }
                                let src_color = source_color(&self.mods[orig_idx].source);
                                let label = match &self.mods[orig_idx].source {
                                    ModSource::Core        => "◆",
                                    ModSource::DLC(_)      => "★",
                                    ModSource::Workshop(_) => "◇",
                                    ModSource::Local       => "◉",
                                };
                                let src_widget = ui.add(
                                    egui::Label::new(RichText::new(label).color(src_color).size(11.0))
                                        .selectable(false)
                                        .sense(Sense::hover())
                                );
                                src_widget.on_hover_text(source_label(&self.mods[orig_idx].source));
                            });
                        });

                        // Колонка: название
                        row.col(|ui| {
                            ui.push_id(orig_idx * 4 + 1, |ui| {
                                ui.painter().rect_filled(ui.max_rect(), 0.0, row_bg);
                                if is_drop_target { paint_drop_line(ui, ui.max_rect()); }
                                let name_color = if is_selected && !is_being_dragged {
                                    Color32::WHITE
                                } else if has_incompat {
                                    theme::ERROR_RED
                                } else if has_missing_deps && self.is_active {
                                    theme::WARNING_AMBER
                                } else if is_being_dragged {
                                    theme::TEXT_MUTED
                                } else {
                                    theme::TEXT_PRIMARY
                                };
                                ui.add(
                                    egui::Label::new(RichText::new(&self.mods[orig_idx].name)
                                        .color(name_color).size(12.0))
                                        .truncate()
                                        .selectable(false),
                                );
                            });
                        });

                        // Колонка: версия
                        row.col(|ui| {
                            ui.push_id(orig_idx * 4 + 2, |ui| {
                                ui.painter().rect_filled(ui.max_rect(), 0.0, row_bg);
                                if is_drop_target { paint_drop_line(ui, ui.max_rect()); }
                                let m = &self.mods[orig_idx];
                                let ver = if !m.version.is_empty() {
                                    m.version.as_str()
                                } else {
                                    m.supported_versions.last().map(String::as_str).unwrap_or("")
                                };
                                ui.add(egui::Label::new(RichText::new(ver)
                                    .color(theme::TEXT_MUTED).size(11.0)).selectable(false));
                            });
                        });

                        // Колонка: предупреждения
                        row.col(|ui| {
                            ui.push_id(orig_idx * 4 + 3, |ui| {
                                let rect = ui.max_rect();
                                ui.painter().rect_filled(rect, 0.0, row_bg);
                                if is_drop_target { paint_drop_line(ui, rect); }
                                if has_incompat {
                                    let warn_widget = ui.add(
                                        egui::Label::new(RichText::new("✕").color(theme::ERROR_RED).size(11.0))
                                            .selectable(false)
                                            .sense(Sense::hover())
                                    );
                                    warn_widget.on_hover_text("Конфликт с активным модом");
                                } else if has_missing_deps && self.is_active {
                                    let warn_widget = ui.add(
                                        egui::Label::new(RichText::new("⚠").color(theme::WARNING_AMBER).size(11.0))
                                            .selectable(false)
                                            .sense(Sense::hover())
                                    );
                                    warn_widget.on_hover_text("Отсутствуют зависимости");
                                }
                            });
                        });

                        let row_resp = row.response();

                        if row_resp.drag_started() {
                            egui::DragAndDrop::set_payload(&ctx, DragPayload { orig_idx });
                        }

                        if is_dragging && row_resp.hovered() && !is_being_dragged {
                            cur_drop_row = Some(row_pos);
                        }

                        if !is_dragging && row_resp.hovered() {
                            cur_hovered = Some(row_pos);
                        }

                        if row_resp.clicked() {
                            *self.selected = Some(orig_idx);
                        }

                        if row_resp.double_clicked() {
                            move_request = Some(if self.is_active {
                                MoveRequest::Deactivate(orig_idx)
                            } else {
                                MoveRequest::Activate(orig_idx)
                            });
                        }

                        row_resp.context_menu(|ui| {
                            ui.set_min_width(180.0);
                            ui.label(RichText::new(&self.mods[orig_idx].name)
                                .color(theme::TEXT_ACCENT).size(11.0).strong());
                            ui.separator();
                            if self.is_active {
                                if ui.button("⬅  Деактивировать").clicked() {
                                    move_request = Some(MoveRequest::Deactivate(orig_idx));
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button("⬆  Переместить вверх").clicked() {
                                    move_request = Some(MoveRequest::MoveUp(orig_idx));
                                    ui.close();
                                }
                                if ui.button("⬇  Переместить вниз").clicked() {
                                    move_request = Some(MoveRequest::MoveDown(orig_idx));
                                    ui.close();
                                }
                            } else if ui.button("➡  Активировать").clicked() {
                                move_request = Some(MoveRequest::Activate(orig_idx));
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("📁  Открыть папку").clicked() {
                                move_request = Some(MoveRequest::OpenFolder(orig_idx));
                                ui.close();
                            }
                        });
                    });
                });

            if is_dragging && ctx.input(|i| i.pointer.primary_released()) {
                if let Some(drop_pos) = cur_drop_row.or(prev_drop_row) {
                    if let Some(payload) = egui::DragAndDrop::payload::<DragPayload>(&ctx) {
                        let p = (*payload).clone();
                        move_request = Some(MoveRequest::DragDrop {
                            orig_idx:  p.orig_idx,
                            to_active: self.is_active,
                            to_pos:    drop_pos,
                        });
                    }
                    egui::DragAndDrop::clear_payload(&ctx);
                }
            }

            ctx.data_mut(|d| d.insert_temp(hover_key, cur_hovered));
            ctx.data_mut(|d| d.insert_temp(drop_key,  cur_drop_row));

            move_request
        });

        result.inner
    }
}

fn paint_drop_line(ui: &Ui, rect: Rect) {
    ui.painter().rect_filled(
        Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), 2.0)),
        0.0,
        theme::BORDER_ACCENT,
    );
}