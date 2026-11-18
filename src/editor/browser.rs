//! Panel de la biblioteca: los videos de la carpeta de clips (y de sus carpetas) y los importados.
//! Un clic elige (con Ctrl se suma, con Shift se elige un tramo) y arrastrar desde cualquier lado elige con un
//! recuadro; doble clic o el "+" de una tarjeta la suma a la línea de tiempo. Lo elegido se mueve a una carpeta
//! o se manda a la Papelera de reciclaje (los importados solo salen de la lista: el archivo no se toca).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use eframe::egui::{self, Align, Align2, Color32, CursorIcon, FontId, Layout, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};

use crate::ui_kit::theme::*;
use crate::ui_kit::window::open_folder;
use super::library::{self, Asset, Folder};
use super::shortcuts::{self, Act};
use super::EditorApp;

/// Alto del título y los datos debajo de la miniatura de una tarjeta.
const CARD_TEXT_H: f32 = 42.0;
const GAP: f32 = 10.0;
const ROW_GAP: f32 = 6.0;
/// Radio del "+" de una tarjeta.
const ADD_R: f32 = 13.0;
/// Con el recuadro de selección a menos de esto del borde de arriba o de abajo, la lista se desplaza sola.
const EDGE: f32 = 40.0;
const AUTO_SCROLL: f32 = 700.0;

#[derive(Default)]
pub(super) struct Browser {
    /// Carpeta que se ve (None = la de los clips, con sus carpetas y los importados).
    pub(super) dir: Option<PathBuf>,
    /// Las carpetas de adentro de la de los clips.
    pub(super) folders: Vec<Folder>,
    /// Lo elegido.
    pub(super) sel: HashSet<PathBuf>,
    /// Desde dónde elige Shift + clic (lugar en la lista de videos).
    anchor: Option<usize>,
    /// Recuadro de selección: dónde empezó (en el contenido de la lista) y lo que estaba elegido antes (con Ctrl).
    marquee: Option<(Pos2, HashSet<PathBuf>)>,
    scroll: f32,
    set_scroll: Option<f32>,
    /// Lo último que se tocó es la biblioteca: Supr borra lo elegido acá y no en la línea de tiempo.
    pub(super) focus: bool,
    dialog: Option<Dialog>,
}

enum Dialog {
    /// Eliminar videos: los de la carpeta de clips van a la Papelera; los importados solo salen de la lista.
    Delete(Vec<PathBuf>),
    /// Eliminar una carpeta con lo que tiene.
    DeleteFolder(PathBuf),
    /// Carpeta nueva (y mover ahí `files`) o cambiarle el nombre a `rename`.
    Name { name: String, files: Vec<PathBuf>, rename: Option<PathBuf>, error: Option<String> },
}

/// "1 clip", "3 clips".
fn count(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{} {many}", fmt_count(n))
    }
}

/// Lo que entra en el renglón de abajo de una tarjeta: tamaño y de dónde viene.
fn asset_card(ui: &mut egui::Ui, rect: Rect, a: &Asset, info: Option<&library::Info>, selected: bool, used: bool) -> (egui::Response, bool) {
    let resp = ui.interact(rect, ui.id().with(("asset", &a.path)), Sense::click());
    let thumb = Rect::from_min_size(rect.min, egui::vec2(rect.width(), rect.height() - CARD_TEXT_H));
    let p = ui.painter();
    let mut shapes = vec![Shape::rect_filled(thumb, 6.0, CONTROL)];
    match info.and_then(|i| i.cover.as_ref()) {
        Some(t) => shapes.push(Shape::image(t.id(), thumb, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE)),
        None => {
            let msg = if info.and_then(|i| i.error.as_ref()).is_some() { "読み取れません" } else { "読み込み中…" };
            shapes.push(ui.fonts(|f| Shape::text(f, thumb.center(), Align2::CENTER_CENTER, msg, FontId::proportional(12.0), MUTED)));
        }
    }
    if let Some(pr) = info.and_then(|i| i.probe.as_ref()) {
        let g = p.layout_no_wrap(super::panels::fmt_len(pr.duration), FontId::proportional(11.5), WHITE);
        let r = Rect::from_min_size(thumb.right_bottom() - g.size() - egui::vec2(14.0, 10.0), g.size() + egui::vec2(8.0, 4.0));
        shapes.push(Shape::rect_filled(r, 4.0, BADGE));
        shapes.push(Shape::galley(r.min + egui::vec2(4.0, 2.0), g, WHITE));
    }
    if used {
        let c = thumb.min + egui::vec2(15.0, 15.0);
        shapes.push(Shape::circle_filled(c, 9.0, WHITE));
        shapes.push(Shape::line(vec![c + egui::vec2(-4.0, 0.0), c + egui::vec2(-1.0, 3.5), c + egui::vec2(4.5, -3.5)], Stroke::new(2.0_f32, BLACK)));
    }
    let hovered = ui.rect_contains_pointer(rect);
    if selected {
        shapes.push(Shape::rect_filled(thumb, 6.0, HOVER_DIM));
        shapes.push(Shape::rect_stroke(thumb, 6.0, Stroke::new(2.0_f32, WHITE)));
    } else if hovered {
        shapes.push(Shape::rect_stroke(thumb, 6.0, Stroke::new(1.0_f32, BORDER)));
    }
    let title = one_line(p, a.title.clone(), FontId::proportional(13.0), rect.width() - 4.0);
    shapes.push(Shape::galley(Pos2::new(rect.left() + 2.0, thumb.bottom() + 6.0), title, TEXT));
    let sub = format!("{}{}", fmt_bytes(a.size as f64), if a.imported { " · importado" } else { "" });
    let sub = one_line(p, sub, FontId::proportional(11.5), rect.width() - 4.0);
    shapes.push(Shape::galley(Pos2::new(rect.left() + 2.0, thumb.bottom() + 25.0), sub, MUTED));
    paint_pressed(ui, rect.center(), press(ui, &resp), PRESS_DEPTH, shapes);
    // "+": sumarla a la línea de tiempo, con el mouse encima o elegida.
    let mut add = false;
    if hovered || selected {
        let c = Pos2::new(thumb.right() - ADD_R - 6.0, thumb.top() + ADD_R + 6.0);
        let r = ui.interact(Rect::from_center_size(c, Vec2::splat(ADD_R * 2.0)), resp.id.with("add"), Sense::click());
        let fill = if r.hovered() { WHITE } else { WHITE.gamma_multiply(0.85) };
        let mut plus = vec![Shape::circle_filled(c, ADD_R, fill)];
        plus.extend(glyph(Glyph::Plus, c, 12.0, BLACK));
        paint_pressed(ui, c, press(ui, &r), PRESS_DEPTH_ICON, plus);
        add = r.on_hover_cursor(CursorIcon::PointingHand).on_hover_text("タイムラインに追加").clicked();
    }
    let resp = resp.on_hover_text(format!("{}\nダブルクリックでタイムラインに追加", a.title));
    (resp, add)
}

fn folder_card(ui: &mut egui::Ui, rect: Rect, f: &Folder) -> egui::Response {
    let resp = ui.interact(rect, ui.id().with(("folder", &f.path)), Sense::click());
    let thumb = Rect::from_min_size(rect.min, egui::vec2(rect.width(), rect.height() - CARD_TEXT_H));
    let p = ui.painter();
    let mut shapes = vec![Shape::rect_filled(thumb, 6.0, CONTROL)];
    if ui.rect_contains_pointer(rect) {
        shapes.push(Shape::rect_stroke(thumb, 6.0, Stroke::new(1.0_f32, BORDER)));
    }
    shapes.extend(glyph(Glyph::Folder, thumb.center(), (thumb.height() * 0.32).clamp(18.0, 40.0), ICON));
    let title = one_line(p, f.name.clone(), FontId::proportional(13.0), rect.width() - 4.0);
    shapes.push(Shape::galley(Pos2::new(rect.left() + 2.0, thumb.bottom() + 6.0), title, TEXT));
    let sub = one_line(p, count(f.count, "video", "videos"), FontId::proportional(11.5), rect.width() - 4.0);
    shapes.push(Shape::galley(Pos2::new(rect.left() + 2.0, thumb.bottom() + 25.0), sub, MUTED));
    paint_pressed(ui, rect.center(), press(ui, &resp), PRESS_DEPTH, shapes);
    resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(format!("{}\nClic para abrirla", f.name))
}

/// Lo que se pidió desde la lista (se hace después de dibujarla).
enum Do {
    Add(Vec<PathBuf>),
    Open(Option<PathBuf>),
    Reveal(PathBuf),
    Move(Vec<PathBuf>, PathBuf),
    Dialog(Dialog),
}

impl EditorApp {
    /// Vuelve a leer la carpeta que se ve (si ya no existe, la de los clips).
    pub(super) fn scan(&mut self) {
        self.imported.retain(|p| p.exists());
        if self.browser.dir.as_ref().is_some_and(|d| !d.is_dir()) {
            self.open_dir(None);
            return;
        }
        let (folders, assets) = library::scan(&self.cfg.output_dir, self.browser.dir.as_deref(), &self.imported);
        self.browser.folders = folders;
        self.assets = assets;
        let shown: HashSet<&Path> = self.assets.iter().map(|a| a.path.as_path()).collect();
        self.browser.sel.retain(|p| shown.contains(p.as_path()));
    }

    /// Muestra otra carpeta (None = la de los clips).
    pub(super) fn open_dir(&mut self, dir: Option<PathBuf>) {
        self.browser.dir = dir;
        self.browser.sel.clear();
        self.browser.anchor = None;
        self.browser.set_scroll = Some(0.0);
        self.watcher.set_dirs(self.watched_dirs());
        self.scan();
    }

    /// Carpetas que se miran por si cambian: la de la app, la de los clips y la que se ve.
    pub(super) fn watched_dirs(&self) -> Vec<PathBuf> {
        let mut v = vec![crate::log::app_dir(), self.cfg.output_dir.clone()];
        v.extend(self.browser.dir.clone());
        v
    }

    /// Lo elegido, en el orden de la lista.
    fn selected_assets(&self) -> Vec<PathBuf> {
        self.assets.iter().filter(|a| self.browser.sel.contains(&a.path)).map(|a| a.path.clone()).collect()
    }

    fn is_used(&self, p: &Path) -> bool {
        self.tracks.iter().any(|t| t.clips.iter().any(|c| c.path == p))
    }

    pub(super) fn library_panel(&mut self, ui: &mut egui::Ui) {
        let panel = ui.max_rect();
        if ui.input(|i| i.pointer.any_pressed()) {
            self.browser.focus = ui.rect_contains_pointer(panel);
        }
        let mut todo: Vec<Do> = Vec::new();
        ui.horizontal(|ui| {
            ui.set_height(32.0);
            match self.browser.dir.clone() {
                Some(dir) => {
                    if icon_only_button(ui, Glyph::Back, Kind::Ghost, "Volver a la biblioteca").clicked() {
                        todo.push(Do::Open(None));
                    }
                    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    ui.add(egui::Label::new(RichText::new(name).family(bold()).size(17.0).color(TEXT)).truncate());
                }
                None => {
                    ui.label(RichText::new("ライブラリ").family(bold()).size(17.0).color(TEXT));
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let tip = format!("任意のフォルダから動画を追加 ({})", shortcuts::short(&self.prefs.keys, Act::Import));
                if icon_button(ui, Glyph::Plus, "インポート", Kind::Secondary, Vec2::ZERO).on_hover_text(tip).clicked() {
                    self.import_dialog();
                }
                if self.browser.dir.is_none() && icon_only_button(ui, Glyph::NewFolder, Kind::Ghost, "新しいフォルダ").clicked() {
                    todo.push(Do::Dialog(Dialog::Name { name: "新しいフォルダ".into(), files: Vec::new(), rename: None, error: None }));
                }
            });
        });
        ui.add_space(10.0);

        // Abajo, lo elegido y qué hacer con eso.
        if !self.browser.sel.is_empty() {
            egui::TopBottomPanel::bottom("lib_sel")
                .frame(egui::Frame::none().inner_margin(egui::Margin { top: 8.0, ..Default::default() }))
                .show_separator_line(false)
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        let n = self.browser.sel.len();
                        ui.label(RichText::new(if n == 1 { "1 elegido".to_string() } else { format!("{} elegidos", fmt_count(n)) }).size(13.0).color(MUTED));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icon_only_button(ui, Glyph::Trash, Kind::Ghost, "削除 (Delete)").clicked() {
                                todo.push(Do::Dialog(Dialog::Delete(self.selected_assets())));
                            }
                            if icon_button(ui, Glyph::Plus, "追加", Kind::Secondary, Vec2::ZERO).on_hover_text("タイムラインに追加").clicked() {
                                todo.push(Do::Add(self.selected_assets()));
                            }
                        });
                    });
                });
        }

        let nf = if self.browser.dir.is_none() { self.browser.folders.len() } else { 0 };
        if nf + self.assets.len() == 0 {
            ui.add_space(24.0);
            if self.browser.dir.is_some() {
                ui.label(RichText::new("このフォルダは空です。").color(TEXT));
                ui.label(RichText::new("クリップを追加するには、ライブラリで選択して右クリックの「移動先」を使ってください。").color(MUTED));
            } else {
                ui.label(RichText::new("まだ動画がありません。").color(TEXT));
                ui.label(RichText::new("プレイ中にクリップを保存するか、「インポート」を押すか、動画をウィンドウにドラッグしてください。").color(MUTED));
            }
        } else {
            self.library_grid(ui, &mut todo);
        }
        for d in todo {
            match d {
                Do::Add(paths) => {
                    for p in paths {
                        self.add_to_timeline(&p);
                    }
                }
                Do::Open(dir) => self.open_dir(dir),
                Do::Reveal(p) => {
                    let _ = std::process::Command::new("explorer").arg(format!("/select,{}", p.display())).spawn();
                }
                Do::Move(paths, dir) => self.move_assets(paths, &dir),
                Do::Dialog(d) => self.browser.dialog = Some(d),
            }
        }
    }

    fn library_grid(&mut self, ui: &mut egui::Ui, todo: &mut Vec<Do>) {
        let nf = if self.browser.dir.is_none() { self.browser.folders.len() } else { 0 };
        let w = ((ui.available_width() - GAP) / 2.0).floor();
        let card = egui::vec2(w, (w * 9.0 / 16.0).round() + CARD_TEXT_H);
        let row_h = card.y + ROW_GAP;
        let n = nf + self.assets.len();
        let used: HashSet<&Path> = self.tracks.iter().flat_map(|t| t.clips.iter().map(|p| p.path.as_path())).collect();
        let mut sa = egui::ScrollArea::vertical().id_salt(("lib", &self.browser.dir)).auto_shrink([false, false]).drag_to_scroll(false);
        if let Some(y) = self.browser.set_scroll.take() {
            sa = sa.vertical_scroll_offset(y);
        }
        let b = &mut self.browser;
        let out = sa.show_viewport(ui, |ui, view| {
            ui.set_height(n.div_ceil(2) as f32 * row_h);
            let origin = ui.max_rect().min;
            let at = |k: usize| Rect::from_min_size(Pos2::new((k % 2) as f32 * (w + GAP), (k / 2) as f32 * row_h), card);
            // Fondo: arrastrar elige con un recuadro (también empezando sobre una tarjeta); un clic ahí no elige nada.
            let bg = ui.interact(view.translate(origin.to_vec2()), ui.id().with("lib_bg"), Sense::click_and_drag());
            let first = (view.top() / row_h).floor().max(0.0) as usize * 2;
            let last = ((view.bottom() / row_h).ceil() as usize * 2).min(n);
            for k in first.max(nf)..last {
                let a = &self.assets[k - nf];
                self.loader.want_cover(&a.path, (a.size, a.modified));
            }
            let info = self.loader.info.lock().unwrap();
            for k in first..last {
                let rect = at(k).translate(origin.to_vec2());
                if k < nf {
                    let f = &b.folders[k];
                    let r = folder_card(ui, rect, f);
                    if r.clicked() {
                        todo.push(Do::Open(Some(f.path.clone())));
                    }
                    r.context_menu(|ui| {
                        if ui.button("開く").clicked() {
                            todo.push(Do::Open(Some(f.path.clone())));
                            ui.close_menu();
                        }
                        if ui.button("名前を変更…").clicked() {
                            todo.push(Do::Dialog(Dialog::Name { name: f.name.clone(), files: Vec::new(), rename: Some(f.path.clone()), error: None }));
                            ui.close_menu();
                        }
                        if ui.button("エクスプローラーで開く").clicked() {
                            open_folder(&f.path);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(RichText::new("フォルダを削除…").color(DANGER)).clicked() {
                            todo.push(Do::Dialog(Dialog::DeleteFolder(f.path.clone())));
                            ui.close_menu();
                        }
                    });
                    continue;
                }
                let ai = k - nf;
                let a = &self.assets[ai];
                let (r, add) = asset_card(ui, rect, a, info.get(&a.path), b.sel.contains(&a.path), used.contains(a.path.as_path()));
                if add {
                    todo.push(Do::Add(if b.sel.contains(&a.path) { self.assets.iter().filter(|x| b.sel.contains(&x.path)).map(|x| x.path.clone()).collect() } else { vec![a.path.clone()] }));
                } else if r.double_clicked() {
                    todo.push(Do::Add(vec![a.path.clone()]));
                } else if r.clicked() {
                    let m = ui.input(|i| i.modifiers);
                    match b.anchor.filter(|_| m.shift) {
                        Some(from) => {
                            let (lo, hi) = (from.min(ai), from.max(ai));
                            if !m.command {
                                b.sel.clear();
                            }
                            b.sel.extend(self.assets[lo..=hi].iter().map(|x| x.path.clone()));
                        }
                        None if m.command => {
                            if !b.sel.remove(&a.path) {
                                b.sel.insert(a.path.clone());
                            }
                            b.anchor = Some(ai);
                        }
                        None => {
                            b.sel = HashSet::from([a.path.clone()]);
                            b.anchor = Some(ai);
                        }
                    }
                }
                if r.secondary_clicked() && !b.sel.contains(&a.path) {
                    b.sel = HashSet::from([a.path.clone()]);
                    b.anchor = Some(ai);
                }
                r.context_menu(|ui| {
                    let paths: Vec<PathBuf> = self.assets.iter().filter(|x| b.sel.contains(&x.path)).map(|x| x.path.clone()).collect();
                    let n = paths.len();
                    let imported = self.assets.iter().filter(|x| b.sel.contains(&x.path) && x.imported).count();
                    let label = if n == 1 { "タイムラインに追加".to_string() } else { format!("{} をタイムラインに追加", count(n, "video", "videos")) };
                    if ui.button(label).clicked() {
                        todo.push(Do::Add(paths.clone()));
                        ui.close_menu();
                    }
                    if n == 1 && ui.button("エクスプローラーで表示").clicked() {
                        todo.push(Do::Reveal(paths[0].clone()));
                        ui.close_menu();
                    }
                    if imported == 0 {
                        ui.menu_button("移動先", |ui| {
                            if b.dir.is_some() && ui.button("ライブラリ").clicked() {
                                todo.push(Do::Move(paths.clone(), self.cfg.output_dir.clone()));
                                ui.close_menu();
                            }
                            for f in b.folders.iter().filter(|f| Some(&f.path) != b.dir.as_ref()) {
                                if ui.button(&f.name).clicked() {
                                    todo.push(Do::Move(paths.clone(), f.path.clone()));
                                    ui.close_menu();
                                }
                            }
                            ui.separator();
                            if ui.button("新しいフォルダ…").clicked() {
                                todo.push(Do::Dialog(Dialog::Name { name: "新しいフォルダ".into(), files: paths.clone(), rename: None, error: None }));
                                ui.close_menu();
                            }
                        });
                    }
                    ui.separator();
                    let del = if imported == n { "ライブラリから削除" } else { "削除…" };
                    if ui.button(RichText::new(del).color(DANGER)).clicked() {
                        todo.push(Do::Dialog(Dialog::Delete(paths.clone())));
                        ui.close_menu();
                    }
                });
            }
            drop(info);

            // Recuadro de selección.
            let pointer = ui.input(|i| i.pointer.interact_pos());
            if bg.drag_started() {
                let start = ui.input(|i| i.pointer.press_origin()).unwrap_or_default() - origin.to_vec2();
                let keep = if ui.input(|i| i.modifiers.command) { b.sel.clone() } else { HashSet::new() };
                b.marquee = Some((start, keep));
            }
            if let (Some((start, keep)), Some(h), true) = (&b.marquee, pointer, bg.dragged()) {
                let r = Rect::from_two_pos(*start, h - origin.to_vec2());
                b.sel = keep.clone();
                b.sel.extend((nf..n).filter(|&k| at(k).intersects(r)).map(|k| self.assets[k - nf].path.clone()));
                let shown = r.translate(origin.to_vec2());
                ui.painter().rect_filled(shown, 2.0, WHITE.gamma_multiply(0.08));
                ui.painter().rect_stroke(shown, 2.0, Stroke::new(1.0_f32, WHITE.gamma_multiply(0.6)));
                // Cerca del borde (o afuera), la lista se desplaza sola.
                let screen = view.translate(origin.to_vec2());
                let push = ((h.y - (screen.bottom() - EDGE)).max(0.0) - (screen.top() + EDGE - h.y).max(0.0)) / EDGE;
                if push != 0.0 {
                    let dt = ui.input(|i| i.stable_dt).min(0.05);
                    b.set_scroll = Some((b.scroll + push.clamp(-1.0, 1.0) * AUTO_SCROLL * dt).max(0.0));
                    ui.ctx().request_repaint();
                }
            }
            if bg.drag_stopped() {
                b.marquee = None;
            }
            if bg.clicked() {
                b.sel.clear();
                b.anchor = None;
            }
        });
        self.browser.scroll = out.state.offset.y;
    }

    /// Hay un diálogo de la biblioteca abierto (los atajos esperan).
    pub(super) fn library_dialog_open(&self) -> bool {
        self.browser.dialog.is_some()
    }

    /// Supr con la biblioteca elegida: pedir confirmación para borrar lo elegido.
    pub(super) fn delete_library_selection(&mut self) -> bool {
        if !self.browser.focus || self.browser.sel.is_empty() {
            return false;
        }
        self.browser.dialog = Some(Dialog::Delete(self.selected_assets()));
        true
    }

    /// No se mueve ni se borra lo que está en la línea de tiempo mientras se exporta (se está leyendo).
    fn busy_with(&mut self, paths: &[PathBuf]) -> bool {
        let busy = self.job.is_some() && paths.iter().any(|p| self.is_used(p));
        if busy {
            self.message = Some((false, "エクスポートが終わるまで待ってください。このクリップは使用中です。".into(), None));
        }
        busy
    }

    fn move_assets(&mut self, paths: Vec<PathBuf>, dir: &Path) {
        if self.busy_with(&paths) {
            return;
        }
        self.release_files(&paths);
        let mut failed = Vec::new();
        for p in &paths {
            match library::move_into(p, dir) {
                Ok(to) => self.moved_file(p, &to),
                Err(e) => {
                    crate::log::log(&format!("biblioteca: no se pudo mover {}: {e}", p.display()));
                    failed.push(library::pretty_name(p));
                }
            }
        }
        self.browser.sel.clear();
        self.scan();
        if !failed.is_empty() {
            self.message = Some((false, format!("No se pudo mover: {}", failed.join(", ")), None));
        }
    }

    fn delete_assets(&mut self, paths: Vec<PathBuf>) {
        let (imported, own): (Vec<PathBuf>, Vec<PathBuf>) = paths.into_iter().partition(|p| self.imported.contains(p));
        if self.busy_with(&own) {
            return;
        }
        self.imported.retain(|p| !imported.contains(p));
        library::save_imported(&self.imported);
        self.release_files(&own);
        let mut gone = Vec::new();
        let mut failed = Vec::new();
        for p in own {
            match library::recycle(&p) {
                Ok(()) => gone.push(p),
                Err(e) => {
                    crate::log::log(&format!("biblioteca: no se pudo eliminar {}: {e}", p.display()));
                    failed.push(library::pretty_name(&p));
                }
            }
        }
        self.forget_files(&gone);
        self.browser.sel.clear();
        self.scan();
        if !failed.is_empty() {
            self.message = Some((false, format!("No se pudo eliminar: {}", failed.join(", ")), None));
        }
    }

    fn delete_folder(&mut self, dir: &Path) {
        let inside: Vec<PathBuf> = library::scan(dir, None, &[]).1.into_iter().map(|a| a.path).collect();
        if self.busy_with(&inside) {
            return;
        }
        self.release_files(&inside);
        match library::recycle(dir) {
            Ok(()) => {
                self.forget_files(&inside);
                if self.browser.dir.as_deref() == Some(dir) {
                    self.open_dir(None);
                }
                self.scan();
            }
            Err(e) => {
                crate::log::log(&format!("biblioteca: no se pudo eliminar {}: {e}", dir.display()));
                self.message = Some((false, "フォルダを削除できませんでした。中のファイルが他のプログラムで開かれている可能性があります。".into(), None));
            }
        }
    }

    /// Crea la carpeta (y mueve ahí `files`) o le cambia el nombre a `rename`. Devuelve el error para mostrar.
    fn name_folder(&mut self, name: &str, files: Vec<PathBuf>, rename: Option<PathBuf>) -> Result<(), String> {
        let name = name.trim();
        if !library::folder_name_ok(name) {
            return Err("Ese nombre no sirve: no puede llevar < > : \" / \\ | ? * ni terminar en punto.".into());
        }
        let to = self.cfg.output_dir.join(name);
        if rename.as_deref() == Some(to.as_path()) {
            return Ok(());
        }
        if to.exists() {
            return Err("Ya hay una carpeta con ese nombre.".into());
        }
        match rename {
            Some(from) => {
                let inside: Vec<PathBuf> = library::scan(&from, None, &[]).1.into_iter().map(|a| a.path).collect();
                if self.busy_with(&inside) {
                    return Ok(());
                }
                self.release_files(&inside);
                let moved = library::rename_folder(&from, &to).map_err(|_| "名前を変更できませんでした。中のファイルが他のプログラムで開かれている可能性があります。".to_string())?;
                for (a, b) in moved {
                    self.moved_file(&a, &b);
                }
                if self.browser.dir.as_deref() == Some(from.as_path()) {
                    self.open_dir(Some(to));
                }
            }
            None => {
                std::fs::create_dir(&to).map_err(|e| format!("No se pudo crear la carpeta: {e}"))?;
                if !files.is_empty() {
                    self.move_assets(files, &to);
                }
            }
        }
        self.scan();
        Ok(())
    }

    /// Los diálogos de la biblioteca: confirmar antes de borrar y elegir el nombre de una carpeta.
    pub(super) fn library_dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.browser.dialog.take() else { return };
        // 0 = sigue abierto, 1 = cancelar, 2 = hacerlo.
        let mut action = 0;
        let closed = match &mut d {
            Dialog::Delete(paths) => {
                let n = paths.len();
                let imported = paths.iter().filter(|p| self.imported.contains(p)).count();
                let own = n - imported;
                let used = paths.iter().filter(|p| self.is_used(p)).count();
                let title = match (own, n) {
                    (0, 1) => "¿Quitar este video de la biblioteca?".to_string(),
                    (0, _) => format!("{} をライブラリから削除しますか？", count(n, "video", "videos")),
                    (_, 1) => "このクリップを削除しますか？".to_string(),
                    _ => format!("{} を削除しますか？", count(n, "video", "videos")),
                };
                let mut lines = Vec::new();
                if own == 1 {
                    lines.push(if n == 1 { "ごみ箱に移動します。後悔したらそこから復元できます。".to_string() } else { "1件をごみ箱に移動します。後悔したらそこから復元できます。".to_string() });
                } else if own > 1 {
                    let rest = "をごみ箱に移動します。後悔したらそこから復元できます。";
                    lines.push(if own == n { format!("S{}", &rest[1..]) } else { format!("{} {rest}", fmt_count(own)) });
                }
                if imported > 0 {
                    lines.push(if imported == 1 { "インポートした項目はリストから外れるだけです。ファイルはそのまま残ります。".to_string() } else { format!("インポートした {} 件はリストから外れるだけです。ファイルはそのまま残ります。", fmt_count(imported)) });
                }
                if used > 0 && own > 0 {
                    lines.push(if used == 1 { "タイムラインにあります。そこからも削除されます。".to_string() } else { format!("{} 件はタイムラインにあります。そこからも削除されます。", fmt_count(used)) });
                }
                let ok = if own == 0 { "削除" } else { "削除" };
                confirm(ctx, &title, &lines, ok, &mut action)
            }
            Dialog::DeleteFolder(dir) => {
                let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let inside = library::scan(dir, None, &[]).1;
                let used = inside.iter().filter(|a| self.is_used(&a.path)).count();
                let mut lines = vec![if inside.is_empty() {
                    "Se manda a la Papelera de reciclaje.".to_string()
                } else {
                    format!("中の {} 件とともにごみ箱に移動します。後悔したらそこから復元できます。", count(inside.len(), "video", "videos"))
                }];
                if used > 0 {
                    lines.push(format!("{} 件はタイムラインにあります。そこからも削除されます。", if used == 1 { "Uno está".to_string() } else { format!("{} están", fmt_count(used)) }));
                }
                confirm(ctx, &format!("フォルダ「{name}」を削除しますか？"), &lines, "削除", &mut action)
            }
            Dialog::Name { name, rename, error, .. } => {
                let title = if rename.is_some() { "名前を変更" } else { "新しいフォルダ" };
                let ok = if rename.is_some() { "保存" } else { "作成" };
                modal(ctx, "lib_name", title, "Se crea dentro de la carpeta de clips.", 460.0, |ui| {
                    modal_body(ui, |ui| {
                        let r = ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY).margin(egui::vec2(10.0, 7.0)).font(FontId::proportional(14.0)));
                        if !r.has_focus() && !r.lost_focus() {
                            r.request_focus();
                        }
                        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            action = 2;
                        }
                        if let Some(e) = error {
                            ui.label(RichText::new(e.as_str()).size(12.5).color(DANGER));
                        }
                    });
                    modal_footer(ui, |ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if button_sized(ui, ok, Kind::Primary, egui::vec2(120.0, 38.0)).clicked() {
                                action = 2;
                            }
                            if button_sized(ui, "キャンセル", Kind::Secondary, egui::vec2(120.0, 38.0)).clicked() {
                                action = 1;
                            }
                        });
                    });
                })
            }
        };
        if closed && action == 0 {
            action = 1;
        }
        match (action, d) {
            (2, Dialog::Delete(paths)) => self.delete_assets(paths),
            (2, Dialog::DeleteFolder(dir)) => self.delete_folder(&dir),
            (2, Dialog::Name { name, files, rename, .. }) => {
                if let Err(e) = self.name_folder(&name, files.clone(), rename.clone()) {
                    self.browser.dialog = Some(Dialog::Name { name, files, rename, error: Some(e) });
                }
            }
            (0, d) => self.browser.dialog = Some(d),
            _ => {}
        }
    }
}

/// Diálogo para confirmar algo que borra: texto, Cancelar y el botón rojo. Devuelve si se cerró con la X o Esc.
fn confirm(ctx: &egui::Context, title: &str, lines: &[String], ok: &str, action: &mut i32) -> bool {
    modal(ctx, "lib_confirm", title, "", 500.0, |ui| {
        modal_body(ui, |ui| {
            for l in lines {
                ui.add(egui::Label::new(RichText::new(l).color(MUTED)).wrap());
            }
        });
        modal_footer(ui, |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button_sized(ui, ok, Kind::Danger, egui::vec2(120.0, 38.0)).clicked() {
                    *action = 2;
                }
                if button_sized(ui, "キャンセル", Kind::Secondary, egui::vec2(120.0, 38.0)).clicked() {
                    *action = 1;
                }
            });
        });
    })
}
