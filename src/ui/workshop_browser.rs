use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::mpsc;

use egui::{Frame, Margin, RichText, Stroke, Vec2};

use crate::app::theme;
use crate::steam::workshop_api::{self, SortOrder, WorkshopItem};

// ─── Async image cache ───────────────────────────────────────────────────────

struct ImageCache {
    textures: HashMap<String, egui::TextureHandle>,
    pending: HashSet<String>,
    tx: mpsc::SyncSender<(String, Vec<u8>)>,
    rx: mpsc::Receiver<(String, Vec<u8>)>,
}

impl ImageCache {
    fn new() -> Self {
        let (tx, rx) = mpsc::sync_channel(64);
        Self { textures: HashMap::new(), pending: HashSet::new(), tx, rx }
    }

    fn request(&mut self, url: &str) {
        if url.is_empty() || self.textures.contains_key(url) || self.pending.contains(url) {
            return;
        }
        self.pending.insert(url.to_string());
        let url_owned = url.to_string();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result: anyhow::Result<Vec<u8>> = (|| {
                let mut buf = Vec::new();
                ureq::get(&url_owned)
                    .set("User-Agent", "Mozilla/5.0")
                    .call()?
                    .into_reader()
                    .read_to_end(&mut buf)?;
                Ok(buf)
            })();
            if let Ok(bytes) = result {
                let _ = tx.try_send((url_owned, bytes));
            }
        });
    }

    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok((url, bytes)) = self.rx.try_recv() {
            self.pending.remove(&url);
            if let Ok(img) = image::load_from_memory(&bytes) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let ci = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
                let tex = ctx.load_texture(&url, ci, egui::TextureOptions::LINEAR);
                self.textures.insert(url, tex);
            }
        }
    }

    fn get(&self, url: &str) -> Option<&egui::TextureHandle> {
        self.textures.get(url)
    }

    fn is_busy(&self) -> bool {
        !self.pending.is_empty()
    }
}

// ─── Состояние поиска ────────────────────────────────────────────────────────

enum FetchState {
    Idle,
    Loading,
    Done(Vec<WorkshopItem>),
    Error(String),
}

// ─── Панель ───────────────────────────────────────────────────────────────────

pub struct WorkshopBrowser {
    search_input: String,
    sort: SortOrder,
    page: u32,
    has_prev: bool,
    has_next: bool,
    state: FetchState,
    fetch_rx: Option<mpsc::Receiver<Result<(Vec<WorkshopItem>, bool), String>>>,
    images: ImageCache,
    /// (id, title) модов в очереди на скачивание
    queue: Vec<(u64, String)>,
    auto_loaded: bool,
}

impl WorkshopBrowser {
    pub fn new() -> Self {
        Self {
            search_input: String::new(),
            sort: SortOrder::Trending,
            page: 1,
            has_prev: false,
            has_next: false,
            state: FetchState::Idle,
            fetch_rx: None,
            images: ImageCache::new(),
            queue: Vec::new(),
            auto_loaded: false,
        }
    }

    /// Отрисовывает окно браузера.
    /// Возвращает `Some(ids)`, когда пользователь нажимает «Скачать» —
    /// список Workshop ID для передачи в SteamCMD.
    pub fn show(&mut self, ctx: &egui::Context, open: &mut bool) -> Option<Vec<u64>> {
        self.images.poll(ctx);
        self.poll_fetch();

        if !self.auto_loaded {
            self.auto_loaded = true;
            self.trigger_fetch();
        }

        if matches!(&self.state, FetchState::Loading) || self.images.is_busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        let mut result = None;
        egui::Window::new("🔍  Steam Workshop — Браузер модов")
            .open(open)
            .collapsible(false)
            .resizable(true)
            .min_width(720.0)
            .min_height(520.0)
            .frame(
                Frame::window(&ctx.global_style())
                    .fill(theme::BG_PANEL)
                    .stroke(Stroke::new(1.0, theme::BORDER_ACCENT)),
            )
            .show(ctx, |ui| {
                result = self.content(ui);
            });

        result
    }

    // ── Polling ───────────────────────────────────────────────────────────────

    fn poll_fetch(&mut self) {
        let Some(rx) = &self.fetch_rx else { return };
        if let Ok(res) = rx.try_recv() {
            self.fetch_rx = None;
            match res {
                Ok((items, has_next)) => {
                    self.has_next = has_next;
                    self.state = FetchState::Done(items);
                }
                Err(e) => self.state = FetchState::Error(e),
            }
        }
    }

    fn trigger_fetch(&mut self) {
        let query = self.search_input.clone();
        let sort = self.sort;
        let page = self.page;
        let (tx, rx) = mpsc::channel();
        self.fetch_rx = Some(rx);
        self.state = FetchState::Loading;
        self.has_prev = page > 1;
        std::thread::spawn(move || {
            let res = workshop_api::fetch_workshop_page(&query, page, sort)
                .map_err(|e| e.to_string());
            let _ = tx.send(res);
        });
    }

    // ── UI ────────────────────────────────────────────────────────────────────

    fn content(&mut self, ui: &mut egui::Ui) -> Option<Vec<u64>> {
        let mut to_download: Option<Vec<u64>> = None;

        // ── Тулбар поиска ────────────────────────────────────────────────────
        Frame::NONE
            .fill(theme::BG_HEADER)
            .inner_margin(Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let resp = ui.add_sized(
                        [280.0, 22.0],
                        egui::TextEdit::singleline(&mut self.search_input)
                            .hint_text("Поиск модов RimWorld..."),
                    );
                    let search = ui
                        .button(RichText::new("🔍").size(12.0))
                        .on_hover_text("Найти")
                        .clicked()
                        || (resp.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                    if search {
                        self.page = 1;
                        self.trigger_fetch();
                    }

                    ui.add_space(6.0);

                    // Сортировка
                    egui::ComboBox::from_id_salt("wsbrowser_sort")
                        .selected_text(
                            RichText::new(self.sort.label())
                                .color(theme::TEXT_MUTED)
                                .size(11.0),
                        )
                        .show_ui(ui, |ui| {
                            for s in SortOrder::ALL {
                                if ui
                                    .selectable_label(
                                        self.sort == s,
                                        RichText::new(s.label()).size(11.0),
                                    )
                                    .clicked()
                                {
                                    self.sort = s;
                                    self.page = 1;
                                    self.trigger_fetch();
                                }
                            }
                        });

                    ui.add_space(8.0);

                    // Пагинация
                    if ui
                        .add_enabled(
                            self.has_prev,
                            egui::Button::new(RichText::new("◀").color(theme::TEXT_MUTED).size(11.0)),
                        )
                        .clicked()
                    {
                        self.page -= 1;
                        self.trigger_fetch();
                    }
                    ui.label(
                        RichText::new(format!("  стр {}  ", self.page))
                            .color(theme::TEXT_MUTED)
                            .size(11.0),
                    );
                    if ui
                        .add_enabled(
                            self.has_next,
                            egui::Button::new(RichText::new("▶").color(theme::TEXT_MUTED).size(11.0)),
                        )
                        .clicked()
                    {
                        self.page += 1;
                        self.trigger_fetch();
                    }

                    // Спиннер загрузки
                    if matches!(&self.state, FetchState::Loading) {
                        ui.add_space(8.0);
                        ui.spinner();
                    }
                });
            });

        ui.add_space(2.0);

        // Снапшот результатов — освобождаем заимствование self.state
        // до того, как начнём мутировать self.images и self.queue.
        let items_snap: Option<Vec<WorkshopItem>> = match &self.state {
            FetchState::Done(v) => Some(v.clone()),
            _ => None,
        };
        let err_msg: Option<String> = match &self.state {
            FetchState::Error(e) => Some(e.clone()),
            _ => None,
        };
        let is_idle    = matches!(&self.state, FetchState::Idle);
        let is_loading = matches!(&self.state, FetchState::Loading);

        // ── Список результатов ────────────────────────────────────────────────
        let queue_height = if self.queue.is_empty() { 0.0 } else { 72.0 };
        let results_h = (ui.available_height() - queue_height - 4.0).max(80.0);

        egui::ScrollArea::vertical()
            .id_salt("wsbrowser_results")
            .max_height(results_h)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                if is_idle || is_loading {
                    if is_idle {
                        ui.add_space(50.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("Нажмите 🔍 для просмотра популярных модов")
                                    .color(theme::TEXT_MUTED)
                                    .size(12.0)
                                    .italics(),
                            );
                        });
                    }
                    return;
                }

                if let Some(e) = err_msg {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(format!("✕ {e}"))
                                .color(theme::ERROR_RED)
                                .size(11.0),
                        );
                    });
                    return;
                }

                let Some(items) = items_snap else { return };

                if items.is_empty() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("Ничего не найдено")
                                .color(theme::TEXT_MUTED)
                                .size(12.0)
                                .italics(),
                        );
                    });
                    return;
                }

                // Запрашиваем изображения
                for item in &items {
                    self.images.request(&item.preview_url);
                }

                // Рисуем карточки
                let w = ui.available_width();
                for item in &items {
                    let in_queue = self.queue.iter().any(|(id, _)| *id == item.id);

                    let row_bg = if in_queue { theme::BG_SELECTED } else { theme::BG_ROW_EVEN };

                    let frame_resp = Frame::NONE
                        .fill(row_bg)
                        .inner_margin(Margin::symmetric(8, 5))
                        .show(ui, |ui| {
                            ui.set_width(w - 16.0);
                            ui.horizontal(|ui| {
                                // ── Превью ──
                                let img_size = Vec2::new(72.0, 72.0);
                                if let Some(tex) = self.images.get(&item.preview_url) {
                                    ui.add(
                                        egui::Image::new(tex)
                                            .fit_to_exact_size(img_size),
                                    );
                                } else {
                                    let (rect, _) = ui.allocate_exact_size(
                                        img_size,
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 4.0, theme::BG_DARK);
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "…",
                                        egui::FontId::monospace(14.0),
                                        theme::TEXT_MUTED,
                                    );
                                }

                                ui.add_space(8.0);

                                // ── Инфо ──
                                ui.vertical(|ui| {
                                    ui.set_width(ui.available_width() - 110.0);
                                    ui.label(
                                        RichText::new(&item.title)
                                            .color(theme::TEXT_PRIMARY)
                                            .size(12.5)
                                            .strong(),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "by {}  •  {} подписчиков  •  ID: {}",
                                            item.author, item.subscribers, item.id
                                        ))
                                        .color(theme::TEXT_MUTED)
                                        .size(10.5),
                                    );
                                });

                                // ── Кнопка добавления ──
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if in_queue {
                                            let btn = egui::Button::new(
                                                RichText::new("✓ В очереди")
                                                    .color(theme::ACTIVE_GREEN)
                                                    .size(11.0),
                                            )
                                            .fill(theme::BG_DARK)
                                            .stroke(Stroke::new(1.0, theme::ACTIVE_GREEN));
                                            if ui
                                                .add(btn)
                                                .on_hover_text("Убрать из очереди")
                                                .clicked()
                                            {
                                                let rid = item.id;
                                                self.queue.retain(|(id, _)| *id != rid);
                                            }
                                        } else {
                                            let btn = egui::Button::new(
                                                RichText::new("+ Добавить")
                                                    .color(theme::TEXT_PRIMARY)
                                                    .size(11.0),
                                            )
                                            .fill(theme::HEADER_LEFT)
                                            .stroke(Stroke::new(1.0, theme::BORDER_ACCENT));
                                            if ui.add(btn).clicked() {
                                                self.queue.push((item.id, item.title.clone()));
                                            }
                                        }
                                    },
                                );
                            });
                        });

                    // Hover highlight
                    if frame_resp.response.hovered() && !in_queue {
                        ui.painter().rect_stroke(
                            frame_resp.response.rect,
                            0.0,
                            Stroke::new(1.0, theme::BORDER),
                            egui::epaint::StrokeKind::Outside,
                        );
                    }

                    ui.add_space(2.0);
                }
            });

        // ── Очередь на скачивание ─────────────────────────────────────────────
        if !self.queue.is_empty() {
            ui.separator();
            Frame::NONE
                .fill(theme::BG_HEADER)
                .inner_margin(Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("Очередь: {}  ", self.queue.len()))
                                .color(theme::TEXT_MUTED)
                                .size(11.0),
                        );

                        // Список тегов модов
                        egui::ScrollArea::horizontal()
                            .id_salt("wsbrowser_queue_tags")
                            .max_height(26.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let snap = self.queue.clone();
                                    for (id, title) in &snap {
                                        let short = if title.len() > 22 {
                                            format!("{}…", &title[..22])
                                        } else {
                                            title.clone()
                                        };
                                        let tag = egui::Button::new(
                                            RichText::new(format!("✕ {short}"))
                                                .color(theme::TEXT_MUTED)
                                                .size(10.5),
                                        )
                                        .fill(theme::BG_DARK)
                                        .stroke(Stroke::new(1.0, theme::BORDER));
                                        if ui
                                            .add(tag)
                                            .on_hover_text(format!("Убрать {id}"))
                                            .clicked()
                                        {
                                            let rid = *id;
                                            self.queue.retain(|(qid, _)| *qid != rid);
                                        }
                                        ui.add_space(3.0);
                                    }
                                });
                            });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let dl_btn = egui::Button::new(
                                RichText::new("⬇  Скачать через SteamCMD")
                                    .color(theme::TEXT_PRIMARY)
                                    .size(11.5),
                            )
                            .fill(theme::HEADER_LEFT)
                            .stroke(Stroke::new(1.0, theme::BORDER_ACCENT));

                            if ui.add(dl_btn).clicked() {
                                to_download = Some(self.queue.iter().map(|(id, _)| *id).collect());
                                self.queue.clear();
                            }

                            ui.add_space(6.0);

                            if ui
                                .button(
                                    RichText::new("✕ Очистить")
                                        .color(theme::TEXT_MUTED)
                                        .size(11.0),
                                )
                                .clicked()
                            {
                                self.queue.clear();
                            }
                        });
                    });
                });
        }

        to_download
    }
}

