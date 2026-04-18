use egui::{Align2, Button, Context, Frame, Margin, RichText, Stroke, Window};

use crate::mod_data::ModEntry;
use crate::app::{theme, AppSettings, SettingsTab};

/// Открывает нативный диалог выбора папки и возвращает путь как строку.
fn pick_folder(title: &str) -> Option<String> {
    rfd::FileDialog::new()
        .set_title(title)
        .pick_folder()
        .map(|p| p.to_string_lossy().into_owned())
}

/// Открывает нативный диалог выбора файла для чтения.
pub fn pick_open_file(title: &str) -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter("Список модов RimWorld", &["xml", "rml", "rws"])
        .add_filter("Все файлы", &["*"])
        .pick_file()
}

/// Открывает нативный диалог выбора файла для сохранения.
pub fn pick_save_file(title: &str) -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter("Список модов (XML)", &["xml"])
        .set_file_name("ModList.xml")
        .save_file()
}

/// Диалог запроса путей при первом запуске.
/// Возвращает `true`, если пользователь подтвердил кнопкой «Открыть».
pub fn open_folder_dialog(ctx: &Context, open: &mut bool, settings: &mut AppSettings) -> bool {
    if !*open { return false; }

    let mut load_requested = false;

    Window::new(RichText::new("📂  Настройка путей").color(theme::TEXT_PRIMARY).size(13.0).strong())
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(480.0)
        .frame(Frame::window(&ctx.global_style())
            .fill(theme::BG_PANEL)
            .stroke(Stroke::new(1.0, theme::BORDER_ACCENT)))
        .show(ctx, |ui| {
            ui.add_space(6.0);

            // ── Папка игры (обязательно) ────────────────────────────────
            required_label(ui, "Папка с игрой:", settings.game_path.is_empty());
            ui.add_space(3.0);
            ui.label(RichText::new("Корневая папка RimWorld (содержит RimWorldWin64.exe или RimWorldLinux)")
                .color(theme::TEXT_MUTED).size(10.5).italics());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let changed = path_edit(ui, &mut settings.game_path,
                    "/path/to/RimWorld", "open_game_path");
                if ui.small_button("…").clicked() {
                    if let Some(p) = pick_folder("Выберите папку игры") {
                        settings.game_path = p;
                    }
                }
                let _ = changed;
            });

            ui.add_space(10.0);

            // ── Папка с модами (обязательно) ────────────────────────────
            required_label(ui, "Папка с локальными модами:", settings.local_mods_path.is_empty());
            ui.add_space(3.0);
            ui.label(RichText::new("Папка Mods/ внутри директории игры")
                .color(theme::TEXT_MUTED).size(10.5).italics());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                path_edit(ui, &mut settings.local_mods_path,
                    "/path/to/RimWorld/Mods", "open_mods_path");
                if ui.small_button("…").clicked() {
                    if let Some(p) = pick_folder("Выберите папку с модами") {
                        settings.local_mods_path = p;
                    }
                }
            });

            ui.add_space(10.0);

            // ── Папка конфигурации (необязательно) ──────────────────────
            ui.label(RichText::new("Папка с конфигурацией (необязательно):")
                .color(theme::TEXT_PRIMARY).size(12.0).strong());
            ui.add_space(3.0);
            ui.label(RichText::new("Содержит ModsConfig.xml — для загрузки и сохранения активных модов")
                .color(theme::TEXT_MUTED).size(10.5).italics());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                path_edit(ui, &mut settings.config_path,
                    "~/.config/unity3d/.../Config", "open_config_path");
                if ui.small_button("…").clicked() {
                    if let Some(p) = pick_folder("Выберите папку конфигурации") {
                        settings.config_path = p;
                    }
                }
            });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                let can_open = !settings.game_path.is_empty() && !settings.local_mods_path.is_empty();
                let ok_color = if can_open { theme::TEXT_PRIMARY } else { theme::TEXT_MUTED };
                let ok_btn = Button::new(
                    RichText::new("  Открыть  ").color(ok_color).size(12.0)
                ).fill(theme::HEADER_LEFT).stroke(Stroke::new(1.0, theme::BORDER_ACCENT));

                let ok_resp = ui.add_enabled(can_open, ok_btn);
                if ok_resp.clicked() {
                    *open = false;
                    load_requested = true;
                }

                ui.add_space(8.0);

                let cancel_btn = Button::new(
                    RichText::new("  Отмена  ").color(theme::TEXT_MUTED).size(12.0)
                ).fill(theme::BG_ROW_EVEN).stroke(Stroke::new(1.0, theme::BORDER));

                if ui.add(cancel_btn).clicked() { *open = false; }
            });
            ui.add_space(4.0);
        });

    load_requested
}

/// Возвращает `true`, если пользователь подтвердил сохранение.
pub fn save_dialog(ctx: &Context, open: &mut bool, mods: &[ModEntry], config_path: &str) -> bool {
    if !*open { return false; }

    let active_count = mods.iter().filter(|m| m.is_active).count();
    let mut save_confirmed = false;

    Window::new(RichText::new("💾  Сохранить список модов").color(theme::TEXT_PRIMARY).size(13.0).strong())
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(360.0)
        .frame(Frame::window(&ctx.global_style())
            .fill(theme::BG_PANEL)
            .stroke(Stroke::new(1.0, theme::BORDER_ACCENT)))
        .show(ctx, |ui| {
            ui.add_space(6.0);

            Frame::NONE
                .fill(theme::BG_DARK)
                .inner_margin(Margin::same(8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Активных:").color(theme::TEXT_MUTED).size(12.0));
                        ui.label(RichText::new(format!("{}", active_count))
                            .color(theme::ACTIVE_GREEN).size(12.0).strong());
                        ui.add_space(12.0);
                        ui.label(RichText::new("Всего:").color(theme::TEXT_MUTED).size(12.0));
                        ui.label(RichText::new(format!("{}", mods.len()))
                            .color(theme::TEXT_PRIMARY).size(12.0).strong());
                    });
                });

            ui.add_space(6.0);

            if config_path.is_empty() {
                Frame::NONE
                    .fill(theme::BG_DARK)
                    .inner_margin(Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.label(RichText::new("⚠  Путь к конфигурации не задан.\nУкажите его в Настройки → Пути.")
                            .color(theme::WARNING_AMBER).size(11.0));
                    });
            } else {
                let target = format!("{}/ModsConfig.xml", config_path);
                ui.label(RichText::new(format!("Запись в: {}", target))
                    .color(theme::TEXT_MUTED).size(10.5).italics());
            }

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let can_save = !config_path.is_empty();
                let save_btn = Button::new(
                    RichText::new("  💾 Сохранить  ").color(theme::TEXT_PRIMARY).size(12.0)
                ).fill(theme::HEADER_RIGHT)
                 .stroke(Stroke::new(1.0, theme::ACTIVE_GREEN.gamma_multiply(0.5)));

                if ui.add_enabled(can_save, save_btn).clicked() {
                    *open = false;
                    save_confirmed = true;
                }

                ui.add_space(8.0);

                let cancel_btn = Button::new(
                    RichText::new("  Отмена  ").color(theme::TEXT_MUTED).size(12.0)
                ).fill(theme::BG_ROW_EVEN).stroke(Stroke::new(1.0, theme::BORDER));

                if ui.add(cancel_btn).clicked() { *open = false; }
            });
            ui.add_space(4.0);
        });

    save_confirmed
}

/// Возвращает `true`, если пользователь нажал «Применить».
pub fn settings_dialog(ctx: &Context, open: &mut bool, settings: &mut AppSettings) -> bool {
    if !*open { return false; }

    let mut applied = false;

    Window::new("⚙ Настройки")
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .movable(true)
        .collapsible(false)
        .resizable(true)
        .min_width(480.0)
        .min_height(300.0)
        .frame(Frame::window(&ctx.global_style())
            .fill(theme::BG_PANEL)
            .stroke(Stroke::new(1.0, theme::BORDER)))
        .show(ctx, |ui| {

            // ── Вкладки ──────────────────────────────────────────────────
            ui.horizontal(|ui| {
                tab_button(ui, "📁  Пути",       settings.active_tab == SettingsTab::Paths,     || settings.active_tab = SettingsTab::Paths);
                tab_button(ui, "🎨  Интерфейс",  settings.active_tab == SettingsTab::Interface,  || settings.active_tab = SettingsTab::Interface);
                tab_button(ui, "⚙  Поведение",   settings.active_tab == SettingsTab::Behavior,   || settings.active_tab = SettingsTab::Behavior);
            });

            ui.separator();
            ui.add_space(6.0);

            match settings.active_tab {
                // ── Пути ─────────────────────────────────────────────────
                SettingsTab::Paths => {
                    path_row_required(ui, "Местоположение игры",
                        "Корневая папка RimWorld (содержит RimWorldWin64.exe или RimWorldLinux)",
                        &mut settings.game_path,
                        "game_path_edit",
                        true);

                    ui.add_space(10.0);

                    path_row_required(ui, "Местоположение локальных модов",
                        "Папка Mods/ внутри директории игры или пользовательская папка",
                        &mut settings.local_mods_path,
                        "mods_path_edit",
                        true);

                    ui.add_space(10.0);

                    path_row(ui, "Местоположение конфигурации",
                        "Папка с ModsConfig.xml (обычно ~/AppData/LocalLow/.../Config)",
                        &mut settings.config_path,
                        "config_path_edit");

                    ui.add_space(10.0);

                    path_row(ui, "Папка SteamCMD (необязательно)",
                        "Базовая папка для SteamCMD; пусто — использовать папку данных приложения",
                        &mut settings.steamcmd_path,
                        "steamcmd_path_edit");
                }

                // ── Интерфейс ────────────────────────────────────────────
                SettingsTab::Interface => {
                    section_header(ui, "ВНЕШНИЙ ВИД");
                    ui.add_space(4.0);
                    checkbox_row(ui, &mut settings.dark_theme,        "Тёмная тема");
                    checkbox_row(ui, &mut settings.show_package_ids,  "Показывать PackageId в списке");
                }

                // ── Поведение ────────────────────────────────────────────
                SettingsTab::Behavior => {
                    section_header(ui, "ПОВЕДЕНИЕ");
                    ui.add_space(4.0);
                    checkbox_row(ui, &mut settings.sort_on_load, "Автосортировка при загрузке");
                    ui.add_space(6.0);
                    section_header(ui, "СОРТИРОВКА");
                    ui.add_space(4.0);
                    checkbox_row(ui, &mut settings.use_community_rules,
                        "Использовать онлайн-базу правил сообщества");
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        ui.label(RichText::new(
                            "Загружает правила loadBefore/loadAfter с GitHub (RimSort Community Rules).\n\
                             Отключите при отсутствии интернета или для оффлайн-режима."
                        ).color(theme::TEXT_MUTED).size(10.5).italics());
                    });
                }
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                let apply = Button::new(
                    RichText::new("  Применить  ").color(theme::TEXT_PRIMARY).size(12.0)
                ).fill(theme::HEADER_LEFT).stroke(Stroke::new(1.0, theme::BORDER_ACCENT));

                if ui.add(apply).clicked() {
                    *open = false;
                    applied = true;
                }

                ui.add_space(8.0);

                let cancel = Button::new(
                    RichText::new("  Отмена  ").color(theme::TEXT_MUTED).size(12.0)
                ).fill(theme::BG_ROW_EVEN).stroke(Stroke::new(1.0, theme::BORDER));

                if ui.add(cancel).clicked() { *open = false; }
            });
            ui.add_space(4.0);
        });

    applied
}

// ─── Вспомогательные виджеты ─────────────────────────────────────────────────

fn tab_button(ui: &mut egui::Ui, label: &str, active: bool, on_click: impl FnOnce()) {
    let fill   = if active { theme::BG_HEADER } else { theme::BG_DARK };
    let color  = if active { theme::TEXT_ACCENT } else { theme::TEXT_MUTED };
    let border = if active { theme::BORDER_ACCENT } else { theme::BORDER };

    let btn = Button::new(RichText::new(label).color(color).size(12.0))
        .fill(fill)
        .stroke(Stroke::new(1.0, border));

    if ui.add(btn).clicked() { on_click(); }
}

fn path_row(ui: &mut egui::Ui, label: &str, hint: &str, value: &mut String, id: &str) {
    path_row_required(ui, label, hint, value, id, false);
}

fn path_row_required(ui: &mut egui::Ui, label: &str, hint: &str, value: &mut String, id: &str, required: bool) {
    required_label(ui, label, required && value.is_empty());
    ui.add_space(2.0);
    ui.label(RichText::new(hint).color(theme::TEXT_MUTED).size(10.5).italics());
    ui.add_space(4.0);

    let spacing = ui.spacing().item_spacing.x;

    ui.horizontal(|ui| {
        if ui.button("…").on_hover_text("Выбрать папку").clicked() {
            if let Some(p) = pick_folder(label) {
                *value = p;
            }
        }
        ui.add_space(spacing);
        ui.add(
            egui::TextEdit::singleline(value)
                .id(egui::Id::new(id))
                .desired_width(ui.available_width())
                .text_color(theme::TEXT_PRIMARY)
                .hint_text("Не задан"),
        );
    });
}

fn checkbox_row(ui: &mut egui::Ui, value: &mut bool, label: &str) {
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.checkbox(value, RichText::new(label).color(theme::TEXT_PRIMARY).size(12.0));
    });
}

fn section_header(ui: &mut egui::Ui, title: &str) {
    Frame::NONE
        .fill(theme::BG_HEADER)
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(title).color(theme::TEXT_MUTED).size(10.0).strong());
        });
}

/// Заголовок поля с опциональным красным маркером обязательности.
fn required_label(ui: &mut egui::Ui, label: &str, is_empty: bool) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(theme::TEXT_PRIMARY).size(12.0).strong());
        if is_empty {
            ui.label(RichText::new("*").color(theme::ERROR_RED).size(12.0).strong())
                .on_hover_text("Обязательное поле");
        }
    });
}

/// Однострочное поле ввода пути, растянутое на всю доступную ширину (без кнопки).
fn path_edit(ui: &mut egui::Ui, value: &mut String, hint: &str, id: &str) -> egui::Response {
    ui.add(
        egui::TextEdit::singleline(value)
            .id(egui::Id::new(id))
            .desired_width(ui.available_width() - 36.0)
            .text_color(theme::TEXT_PRIMARY)
            .hint_text(hint),
    )
}
