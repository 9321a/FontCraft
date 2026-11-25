// No console window behind FontCraft on Windows.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod finder;
mod gridview;
mod keys;
mod fonts;

use eframe::egui::{
    self, Align2, Color32, ColorImage, FontId, Margin, Pos2, Rect, Sense, Stroke, TextureHandle,
    TextureOptions, Vec2,
};
use fonts::{Library, Source};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

mod i18n;
mod i18n_extra;
mod i18n_extra2;
mod i18n_extra3;
mod i18n_extra4;
mod i18n_extra5;
mod i18n_extra5a;
mod i18n_extra5b;
mod i18n_extra5c;
mod i18n_extra6;
mod i18n_extra7;
mod i18n_extra7a;
mod i18n_extra7b;
mod i18n_extra8;
mod i18n_extra9;
mod i18n_extra10;
mod fcl;
mod paths;
mod menus;
mod metrics;
mod watch;
mod glyphtools;
mod dragout;
mod print;
#[cfg(any(target_os = "windows", all(target_os = "linux", not(target_os = "android"))))]
mod printsys;
#[cfg(any(target_os = "windows", all(target_os = "linux", not(target_os = "android"))))]
mod winauto;
mod appicon;
mod oslook;
mod gfonts;
mod background;
mod importdlg;
mod autoact;
mod presscraft;
mod mechanics;
mod langs;
mod activate;
mod libs;
mod libscan;
mod libui;
mod treemenu;
mod fontreq;
mod docfonts;
mod missing;
mod nativemenu;
mod uiicons;
mod ot;
mod panel;
mod settings;
use i18n::K;
use settings::{Pal, Settings, SettingsTab, ThemeChoice};

fn presets(lang: usize) -> [(&'static str, &'static str); 5] {
    [
    (i18n::tr(lang, K::Sentence), i18n::tr(lang, K::Pangram)),
    ("Aa", "Aa"),
    ("ABC", "ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
    ("abc", "abcdefghijklmnopqrstuvwxyz"),
    ("123", "0123456789 !?&@#%"),
    ]
}

fn class_label(lang: usize, class: fonts::Class, monospaced: bool) -> &'static str {
    if monospaced {
        if lang == 1 { "Monoespaciada" } else { "Monospaced" }
    } else {
        match class {
            fonts::Class::Sans => "Sans Serif",
            fonts::Class::Serif => "Serif",
            fonts::Class::Slab => "Slab Serif",
            fonts::Class::Script => "Script",
            fonts::Class::Blackletter => "Blackletter",
            fonts::Class::Display => "Display",
            fonts::Class::Symbol => if lang == 1 { "Símbolos" } else { "Symbols" },
            fonts::Class::Other => if lang == 1 { "Otras" } else { "Other" },
        }
    }
}

fn days_to_ymd(days: u64) -> (u32, u32, u32) {
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m, d)
}

/// Lets the window's own words show Chinese, Japanese, Korean and other scripts,
/// using fonts that are already on the computer (read in place, not copied).
fn install_fallback_fonts(ctx: &egui::Context) {
    let candidates = [
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/AppleSDGothicNeo.ttc",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/Library/Fonts/Arial Unicode.ttf",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\malgun.ttf",
    ];
    let mut defs = egui::FontDefinitions::default();
    let mut added = 0;
    // The system's own interface font comes first (San Francisco on macOS, Segoe UI on Windows…).
    if let Some(path) = oslook::ui_font_path() {
        if let Ok(f) = std::fs::File::open(&path) {
            if let Ok(map) = unsafe { memmap2::Mmap::map(&f) } {
                let bytes: &'static [u8] = &Box::leak(Box::new(map))[..];
                defs.font_data.insert("system-ui".into(), std::sync::Arc::new(egui::FontData::from_static(bytes)));
                defs.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "system-ui".into());
                added += 1;
            }
        }
    }
    for (i, path) in candidates.iter().enumerate() {
        let Ok(f) = std::fs::File::open(path) else { continue };
        let Ok(map) = (unsafe { memmap2::Mmap::map(&f) }) else { continue };
        let bytes: &'static [u8] = &Box::leak(Box::new(map))[..];
        let name = format!("fallback-{i}");
        defs.font_data.insert(name.clone(), std::sync::Arc::new(egui::FontData::from_static(bytes)));
        for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            defs.families.entry(fam).or_default().push(name.clone());
        }
        added += 1;
    }
    if added > 0 {
        ctx.set_fonts(defs);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum View {
    All,
    Favorites,
    Source(Source),
    Mono,
    Class(fonts::Class),
    /// One of the person's own libraries.
    Lib(usize),
    /// A list (subfolder) inside one of the person's libraries.
    LibSet(usize, usize),
}

#[derive(Clone, Copy, PartialEq)]
enum Row {
    Family(usize),
    Face(usize, usize),
}

/// What a cached picture shows: (kind, font face, text, pixel size, width, features).
type TexKey = (u8, usize, String, u32, u32, u64);

struct Cached {
    tex: Option<TextureHandle>,
    size: Vec2,
    drew: bool,
    missing: bool,
    last: u64,
}

struct App {
    lib: Library,
    view: View,
    search: String,
    preview: String,
    size: f32,
    selected: Option<Row>,
    expanded: HashSet<usize>,
    favorites: HashSet<String>,
    /// Font files on only until FontCraft quits ("Activate Temporarily").
    temp_active: HashSet<PathBuf>,
    /// Sidebar sections folded away.
    folded: HashSet<String>,
    /// Families the person added to a smart list by hand (list key → family names).
    smart_extra: HashMap<String, HashSet<String>>,
    cache: HashMap<TexKey, Cached>,
    detail_tab: panel::DetailTab,
    /// OpenType features switched on or off by the person (missing = the usual default).
    features: HashMap<[u8; 4], bool>,
    feature_open: HashSet<String>,
    glyph_sel: Option<u16>,
    face_data: Option<panel::FaceData>,
    frame: u64,
    screenshot: Option<PathBuf>,
    shot_requested: bool,
    /// When --screenshot started and when the picture was last asked for (seconds).
    shot_started: Option<f64>,
    shot_asked_at: f64,
    /// The window picture the system hands back for --screenshot.
    shot_image: std::sync::Arc<std::sync::Mutex<Option<std::sync::Arc<egui::ColorImage>>>>,
    settings: Settings,
    settings_open: bool,
    settings_tab: SettingsTab,
    pal: Pal,
    libraries: Vec<libs::UserLibrary>,
    /// Font files the person switched on (loaded for other apps).
    active: HashSet<PathBuf>,
    new_lib: Option<libs::NewLib>,
    pub(crate) folder_picker_rx: Option<std::sync::mpsc::Receiver<Option<PathBuf>>>,
    /// A short message shown in the status bar (for example when activation fails).
    notice: Option<String>,
    /// Which families each library's lists hold, and each library's order.
    members: Vec<libs::LibMembers>,
    /// Libraries whose lists are folded away in the sidebar.
    lib_collapsed: HashSet<usize>,
    gf: gfonts::GState,
    /// Whether the left (lists) and right (font details) panels are unfolded.
    left_open: bool,
    right_open: bool,
    /// Waiting for the keys of a new "open FontCraft" shortcut.
    recording_hotkey: bool,
    import_dlg: Option<importdlg::ImportDlg>,
    autoact: autoact::AutoAct,
    bg: background::Background,
    /// Started at login with --background: the menu bar asterisk shows even if the setting is off.
    start_hidden: bool,
    /// A panel edge or tab being dragged to resize: (left panel?, width when the drag began).
    resizing: Option<(bool, f32)>,
    /// How tall each font's sample is, in ems (top of the highest letter to the bottom of the
    /// lowest, at least the font's own line), by (face, text).
    extents: HashMap<(usize, String), f32>,
    /// Settings › General › Share: the radio buttons (Some(true) = all users, Some(false) = one
    /// user, None = not chosen yet), the chosen user, and the last result.
    share_all: Option<bool>,
    share_user: String,
    share_status: Option<(bool, String)>,
    users: Option<Vec<String>>,
    /// Text to put on the clipboard this frame.
    clip: Option<String>,
    /// Glyphs page tools: the character shown in the info window (face, glyph), the character
    /// whose fonts are marked in the list (and which families have it), and a similar-shapes search.
    char_info: Option<(usize, u16)>,
    mark_char: Option<char>,
    mark_cache: HashMap<usize, bool>,
    similar: Option<glyphtools::Similar>,
    /// The window's content view (macOS), for dragging glyphs out to other apps.
    ns_view: *mut std::ffi::c_void,
    /// Started in the background (the menu bar asterisk kept, or --background at login): the window
    /// eframe shows after its first frame is put away again as soon as it is up.
    hide_at_start: bool,
    /// The details panel's floating card this frame (screen points): the glyph preview sits
    /// against its left edge, centred on it.
    details_card: Option<Rect>,
    /// A drag out of the window was already started for the glyph being dragged.
    drag_out_tried: bool,
    print: Option<print::PrintDlg>,
    /// The window picking one of a big library's lists (right-click › Add to Library / Folder).
    list_pick: Option<menus::ListPick>,
    /// Glyph size on the Glyphs page (72 to start; ±2 between 56 and 128).
    glyph_px: f32,
    /// A deletion waiting for the person to confirm.
    confirm: Option<menus::Confirm>,
    /// What each item of the native menu on screen does (by item id).
    native_menu: HashMap<String, menus::MenuAction>,
    /// The Missing Fonts window and the documents it has read.
    missing: missing::MissingState,
    /// Glyph metrics pictures (hover preview, character info), by face, glyph and pixel size.
    metrics_cache: HashMap<(usize, u16, u32), std::rc::Rc<metrics::GlyphMetrics>>,
    /// FontCraft's icon for the sidebar, in the version for the current look (dark or light).
    logo: Option<(bool, egui::TextureHandle)>,
    /// What the search box looks through, per family (see search_text).
    search_index: std::cell::OnceCell<Vec<String>>,
    /// Glyphs page filter: a character typed (or U+XXXX) and a kind of character (RightFont's
    /// "SELF.character == %@" / "SELF.unicodeCategory == %@", strings at 0x10029147c / 0x100291461).
    glyph_find: String,
    glyph_kind: usize,
    /// Watches the libraries' and folders' files (RightFont's RFFileWatcher), the folders it was made
    /// for, and when to read them again after a change (changes come in bursts while files copy).
    watcher: Option<watch::Watcher>,
    watched: Vec<PathBuf>,
    reread_at: Option<f64>,
    /// The fonts being read again in the background, and whether to read once more after.
    scanning: Option<std::sync::Arc<std::sync::Mutex<Option<fonts::Library>>>>,
    scan_again: bool,
    /// The app icon shown now (dark tile or light), and when the system's look was last checked.
    icon_dark: Option<bool>,
    icon_checked: f64,
}

/// What happened to a panel tab this frame.
enum TabAction {
    None,
    /// Clicked: fold or unfold the panel.
    Toggle,
    /// Held and dragged: the pointer moved this far sideways since the drag began.
    Drag(f32),
    DragEnd,
}

/// One of the toolbar's view switches (grid, list), drawn as before: a soft square under the pointer,
/// accent-outlined when it's the view in use. Returns whether it was clicked.
fn view_button(ui: &mut egui::Ui, pal: &Pal, on: bool, tip: &str, draw: impl FnOnce(&egui::Painter, Pos2, Color32)) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::click());
    if resp.hovered() || on {
        ui.painter().rect_filled(r, 4.0, if on { pal.hover } else { pal.hover.gamma_multiply(0.5) });
    }
    if on {
        ui.painter().rect_stroke(r, 4.0, Stroke::new(1.0, pal.accent), eframe::egui::StrokeKind::Inside);
    }
    draw(ui.painter(), r.center(), if on { pal.accent } else { pal.muted });
    resp.on_hover_text(tip).clicked()
}

/// A magnifying glass, where the system has no symbol for it: a ring and a short handle.
fn paint_magnifier(p: &egui::Painter, c: Pos2, col: Color32) {
    let st = Stroke::new(1.5_f32, col);
    let ring = c + Vec2::new(-1.2, -1.2);
    p.circle_stroke(ring, 4.6, st);
    p.line_segment([ring + Vec2::new(3.3, 3.3), ring + Vec2::new(6.6, 6.6)], Stroke::new(1.9_f32, col));
}

/// The system symbol Finder-style rows use for each sidebar entry (SF Symbols names).
fn view_symbol(view: View) -> &'static str {
    match view {
        View::All => "textformat",
        View::Favorites => "star",
        View::Source(Source::System) => "gearshape",
        View::Source(Source::Computer) => "desktopcomputer",
        View::Source(Source::User) => "person",
        View::Source(Source::Google) => "globe",
        View::Source(Source::Adobe) => "a.square",
        View::Source(_) => "books.vertical",
        View::Mono => "chevron.left.forwardslash.chevron.right",
        View::Class(fonts::Class::Sans) => "textformat.alt",
        View::Class(fonts::Class::Serif) => "character",
        View::Class(fonts::Class::Slab) => "a.square.fill",
        View::Class(fonts::Class::Script) => "signature",
        View::Class(fonts::Class::Blackletter) => "scroll",
        View::Class(fonts::Class::Display) => "sparkles",
        View::Class(fonts::Class::Symbol) => "staroflife",
        View::Class(_) => "textformat",
        View::Lib(_) | View::LibSet(..) => "folder",
    }
}

/// FontCraft's own drawing of a sidebar entry's picture, where the system has no symbol.
fn paint_view_icon(p: &egui::Painter, view: View, c: Pos2, icol: Color32) {
    let st = Stroke::new(1.3_f32, icol);
    match view {
        View::All => {
            for (dx, dy) in [(-3.0, -3.0), (3.0, -3.0), (-3.0, 3.0), (3.0, 3.0)] {
                p.rect_filled(Rect::from_center_size(c + Vec2::new(dx, dy), Vec2::splat(4.5)), 1.0, icol);
            }
        }
        View::Favorites => {
            p.text(c, Align2::CENTER_CENTER, "★", FontId::proportional(15.0), icol);
        }
        View::Source(Source::Google) => gfonts::paint_mark(p, c, icol),
        View::Source(Source::Adobe) => {
            p.rect_stroke(Rect::from_center_size(c, Vec2::splat(13.0)), 3.0, st, eframe::egui::StrokeKind::Inside);
            p.text(c + Vec2::new(0.0, 0.5), Align2::CENTER_CENTER, "A", FontId::proportional(10.5), icol);
        }
        View::Source(_) => {
            p.rect_stroke(Rect::from_center_size(c, Vec2::new(13.0, 10.0)), 2.0, st, eframe::egui::StrokeKind::Inside);
            p.line_segment([c + Vec2::new(-3.0, 7.0), c + Vec2::new(3.0, 7.0)], st);
        }
        View::Mono => {
            for dy in [-4.0, 0.0, 4.0] {
                p.line_segment([c + Vec2::new(-6.0, dy), c + Vec2::new(6.0, dy)], st);
            }
        }
        View::Class(fonts::Class::Serif) | View::Class(fonts::Class::Sans) => {
            // A capital T, with little feet for "serif".
            let st = Stroke::new(1.6_f32, icol);
            p.line_segment([c + Vec2::new(-5.5, -5.5), c + Vec2::new(5.5, -5.5)], st);
            p.line_segment([c + Vec2::new(0.0, -5.5), c + Vec2::new(0.0, 5.5)], st);
            if view == View::Class(fonts::Class::Serif) {
                p.line_segment([c + Vec2::new(-2.8, 5.5), c + Vec2::new(2.8, 5.5)], st);
                p.line_segment([c + Vec2::new(-5.5, -5.5), c + Vec2::new(-5.5, -3.0)], st);
                p.line_segment([c + Vec2::new(5.5, -5.5), c + Vec2::new(5.5, -3.0)], st);
            }
        }
        View::Class(fonts::Class::Slab) => {
            // A capital T with heavy block feet.
            let st = Stroke::new(1.6_f32, icol);
            p.line_segment([c + Vec2::new(-5.5, -5.5), c + Vec2::new(5.5, -5.5)], st);
            p.line_segment([c + Vec2::new(0.0, -5.5), c + Vec2::new(0.0, 5.5)], st);
            p.rect_filled(Rect::from_center_size(c + Vec2::new(0.0, 5.0), Vec2::new(6.5, 2.4)), 0.0, icol);
            p.rect_filled(Rect::from_min_size(c + Vec2::new(-6.3, -6.5), Vec2::new(2.4, 4.0)), 0.0, icol);
            p.rect_filled(Rect::from_min_size(c + Vec2::new(3.9, -6.5), Vec2::new(2.4, 4.0)), 0.0, icol);
        }
        View::Class(fonts::Class::Blackletter) => {
            // Broken, angular strokes like a pen-drawn black letter.
            let st = Stroke::new(1.6_f32, icol);
            let pts = vec![
                c + Vec2::new(-5.5, 6.0), c + Vec2::new(-5.5, -3.0), c + Vec2::new(-2.5, -6.0), c + Vec2::new(0.0, -3.5),
                c + Vec2::new(2.5, -6.0), c + Vec2::new(5.5, -3.0), c + Vec2::new(5.5, 6.0),
            ];
            p.add(egui::Shape::line(pts, st));
            p.line_segment([c + Vec2::new(0.0, -3.5), c + Vec2::new(0.0, 4.0)], st);
        }
        View::Class(fonts::Class::Display) => {
            // A star: decorative type.
            let pts: Vec<Pos2> = (0..10)
                .map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                    c + Vec2::angled(a) * if i % 2 == 0 { 7.0 } else { 3.0 }
                })
                .collect();
            p.add(egui::Shape::closed_line(pts, Stroke::new(1.4_f32, icol)));
        }
        View::Class(fonts::Class::Script) => {
            let st = Stroke::new(1.6_f32, icol);
            let pts = [c + Vec2::new(-6.0, 5.0), c + Vec2::new(-1.0, -12.0), c + Vec2::new(3.0, 9.0), c + Vec2::new(6.5, -3.0)];
            p.add(egui::epaint::CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, st));
        }
        View::Class(_) => {
            p.circle_stroke(c, 6.0, st);
            let d = 3.2;
            p.add(egui::Shape::convex_polygon(
                vec![c + Vec2::new(0.0, -d), c + Vec2::new(d, 0.0), c + Vec2::new(0.0, d), c + Vec2::new(-d, 0.0)],
                icol,
                Stroke::NONE,
            ));
        }
        View::Lib(_) | View::LibSet(..) => {}
    }
}

/// A small tab sitting on a panel's edge. `on_left` is the left panel's tab.
/// Click it to fold/unfold the panel; hold it and drag sideways to make the panel wider or narrower.
#[allow(clippy::too_many_arguments)]
fn panel_tab(ctx: &egui::Context, id: &str, edge_x: f32, on_left: bool, open: bool, pal: &Pal, hide: &str, show: &str) -> TabAction {
    let screen = ctx.content_rect();
    let size = Vec2::new(14.0, 56.0);
    // The tab sticks out from the panel into the font list.
    let x = if on_left { edge_x } else { edge_x - size.x };
    let pos = Pos2::new(x, screen.center().y - size.y / 2.0);
    let mut action = TabAction::None;
    egui::Area::new(egui::Id::unique(id)).order(egui::Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
        let (r, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        if resp.dragged() && open {
            ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if resp.drag_stopped() {
            action = TabAction::DragEnd;
        } else if resp.dragged() && open {
            let start = ctx.input(|i| i.pointer.press_origin()).unwrap_or_default();
            let now = ctx.input(|i| i.pointer.interact_pos()).unwrap_or(start);
            action = TabAction::Drag(now.x - start.x);
        }
        let rounding = if on_left {
            egui::CornerRadius { nw: 0, sw: 0, ne: 7, se: 7 }
        } else {
            egui::CornerRadius { nw: 7, sw: 7, ne: 0, se: 0 }
        };
        let fill = if resp.hovered() { pal.accent_soft } else { pal.sidebar };
        ui.painter().rect(r, rounding, fill, Stroke::new(1.0_f32, pal.line), eframe::egui::StrokeKind::Inside);
        // The arrow points where the panel will go.
        let points_left = on_left == open;
        let c = r.center();
        let col = if resp.hovered() { pal.accent } else { pal.muted };
        let st = Stroke::new(1.6_f32, col);
        let (a, b) = if points_left { (2.0, -2.0) } else { (-2.0, 2.0) };
        ui.painter().line_segment([c + Vec2::new(a, -5.0), c + Vec2::new(b, 0.0)], st);
        ui.painter().line_segment([c + Vec2::new(b, 0.0), c + Vec2::new(a, 5.0)], st);
        if resp.on_hover_text(if open { hide } else { show }).clicked() {
            action = TabAction::Toggle;
        }
    });
    action
}

/// A thin strip along a panel's edge that can be dragged to resize it. Returns the sideways
/// movement since the drag began, or None when it isn't being dragged.
fn panel_edge(ctx: &egui::Context, id: &str, edge_x: f32) -> (Option<f32>, bool) {
    let screen = ctx.content_rect();
    let rect = Rect::from_min_max(Pos2::new(edge_x - 3.0, screen.top() + 50.0), Pos2::new(edge_x + 3.0, screen.bottom() - 26.0));
    let mut out = (None, false);
    egui::Area::new(egui::Id::unique(id)).order(egui::Order::Middle).fixed_pos(rect.min).show(ctx, |ui| {
        let (_, resp) = ui.allocate_exact_size(rect.size(), Sense::drag());
        if resp.hovered() || resp.dragged() {
            ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if resp.dragged() {
            let start = ctx.input(|i| i.pointer.press_origin()).unwrap_or_default();
            let now = ctx.input(|i| i.pointer.interact_pos()).unwrap_or(start);
            out.0 = Some(now.x - start.x);
        }
        out.1 = resp.drag_stopped();
    });
    out
}

/// The details panel's widest size.
const DETAILS_MAX_W: f32 = 1000.0;
/// Room inside the details card on either side.
const DETAILS_PAD: f32 = 12.0;

impl App {
    /// The details panel's narrowest size: four columns of glyphs on its Glyphs page at the glyph
    /// size chosen there (each cell is 1.6 × the glyph size), plus the panel's margins (16 px each
    /// side), the grid's 4 px and room for the scroll bar.
    pub(crate) fn details_min_w(&self) -> f32 {
        // Four glyph columns, the card's inset and its inner room on both sides.
        (4.0 * self.glyph_px * 1.6 + 4.0 + 2.0 * (finder::CARD_INSET as f32 + DETAILS_PAD) + 12.0).min(DETAILS_MAX_W)
    }
}

/// The sidebar sections folded away, one key per line.
fn folded_path() -> PathBuf {
    paths::data_dir().join("sidebar-folded.txt")
}

impl App {
    /// Folds a sidebar section away, or opens it again (remembered).
    pub(crate) fn toggle_folded(&mut self, key: &str) {
        if !self.folded.remove(key) {
            self.folded.insert(key.to_string());
        }
        let mut v: Vec<&String> = self.folded.iter().collect();
        v.sort();
        let _ = std::fs::write(folded_path(), v.into_iter().cloned().collect::<Vec<_>>().join("\n"));
    }
}

fn favorites_path() -> PathBuf {
    paths::data_dir().join("favorites.txt")
}

/// Families added to smart lists by hand: one "list key<TAB>family" per line.
fn smart_extra_path() -> PathBuf {
    paths::data_dir().join("smart-lists.txt")
}

fn load_smart_extra() -> HashMap<String, HashSet<String>> {
    let mut m: HashMap<String, HashSet<String>> = HashMap::new();
    for line in std::fs::read_to_string(smart_extra_path()).unwrap_or_default().lines() {
        if let Some((k, fam)) = line.split_once('\t') {
            m.entry(k.to_string()).or_default().insert(fam.to_string());
        }
    }
    m
}

impl App {
    fn new(cc: &eframe::CreationContext, screenshot: Option<PathBuf>) -> Self {
        cc.egui_ctx.all_styles_mut(|s| {
            s.spacing.item_spacing = Vec2::new(8.0, 6.0);
            s.spacing.button_padding = Vec2::new(10.0, 4.0);
        });

        install_fallback_fonts(&cc.egui_ctx);
        cc.egui_ctx.all_styles_mut(oslook::apply_spacing);
        // Test pictures capture one window, so dialogs are drawn inside it then.
        if screenshot.is_some() {
            cc.egui_ctx.set_embed_viewports(true);
        }
        let mut settings = Settings::load();
        // FONTCRAFT_VIEW (test pictures): grid, samples or list, without saving it.
        if screenshot.is_some() {
            match std::env::var("FONTCRAFT_VIEW").as_deref() {
                Ok("grid") => settings.view_mode = settings::ViewMode::Grid,
                Ok("samples") => settings.view_mode = settings::ViewMode::Samples,
                Ok("list") => settings.view_mode = settings::ViewMode::List,
                _ => {}
            }
        }
        let libraries = libs::load_libraries();
        // FontCraft's libraries show FontCraft's library icon in the file manager (Windows, Linux).
        {
            let dirs: Vec<PathBuf> = libraries.iter().filter(|l| l.kind == libs::LibKind::Fcl).map(|l| l.path.clone()).collect();
            std::thread::spawn(move || dirs.iter().for_each(|d| fcl::mark_as_library(d)));
        }
        let lib_paths: Vec<PathBuf> = libraries.iter().map(|l| l.path.clone()).collect();
        let lib = fonts::scan(&lib_paths);
        let favorites = std::fs::read_to_string(favorites_path())
            .map(|s| s.lines().filter(|l| !l.is_empty()).map(String::from).collect())
            .unwrap_or_default();
        let mut selected = if lib.families.is_empty() { None } else { Some(Row::Family(0)) };
        // Lets the automatic checks open a particular family.
        if let Ok(name) = std::env::var("FONTCRAFT_SELECT") {
            if let Some(i) = lib.families.iter().position(|f| f.name == name) {
                selected = Some(Row::Family(i));
            }
        }
        // Lets the automatic checks open a particular list.
        let view = match std::env::var("FONTCRAFT_VIEW").as_deref() {
            Ok("serif") => View::Class(fonts::Class::Serif),
            Ok("sans") => View::Class(fonts::Class::Sans),
            Ok("script") => View::Class(fonts::Class::Script),
            Ok("symbol") => View::Class(fonts::Class::Symbol),
            Ok("slab") => View::Class(fonts::Class::Slab),
            Ok("blackletter") => View::Class(fonts::Class::Blackletter),
            Ok("display") => View::Class(fonts::Class::Display),
            Ok("lib") if !libraries.is_empty() => View::Lib(libraries.len() - 1),
            Ok("set") if libraries.last().is_some_and(|l| !l.sets.is_empty()) => View::LibSet(libraries.len() - 1, 0),
            _ => View::All,
        };
        let new_lib = std::env::var("FONTCRAFT_NEW_LIBRARY")
            .ok()
            .map(libs::NewLib::new)
            .or_else(|| std::env::var("FONTCRAFT_NEW_FCL").ok().map(|_| libs::NewLib::library()));
        let share_choice = fcl::ShareWith::from_setting(&settings.share);
        let start_hidden = screenshot.is_none() && std::env::args().any(|a| a == "--background");
        // "Keep FontCraft in the menu bar" on (or started at login): FontCraft starts in the
        // background, only its menu bar asterisk showing, until the asterisk is clicked.
        let hide_at_start = starts_in_background(&settings, screenshot.is_some());
        let bg = background::Background::default();
        background::set_keep_in_menu_bar(settings.background || start_hidden);
        background::set_in_background(hide_at_start);
        let mut app = App {
            lib,
            view,
            libraries,
            active: libs::load_active(),
            new_lib,
            folder_picker_rx: None,
            notice: None,
            members: Vec::new(),
            lib_collapsed: HashSet::new(),
            gf: gfonts::GState { open: std::env::var("FONTCRAFT_GOOGLE").is_ok(), ..Default::default() },
            left_open: !cfg!(target_os = "android") && std::env::var("FONTCRAFT_LEFT").as_deref() != Ok("closed"),
            right_open: !cfg!(target_os = "android") && std::env::var("FONTCRAFT_RIGHT").as_deref() != Ok("closed"),
            recording_hotkey: false,
            import_dlg: if std::env::var("FONTCRAFT_IMPORT_FONTS").is_ok() { Some(importdlg::ImportDlg::default()) } else { None },
            autoact: autoact::AutoAct::default(),
            bg,
            start_hidden,
            resizing: None,
            extents: HashMap::new(),
            share_user: match &share_choice {
                Some(fcl::ShareWith::User(u)) => u.clone(),
                _ => String::new(),
            },
            share_all: share_choice.as_ref().map(|s| *s == fcl::ShareWith::Everyone),
            share_status: None,
            users: None,
            clip: None,
            char_info: None,
            mark_char: None,
            mark_cache: HashMap::new(),
            similar: None,
            ns_view: std::ptr::null_mut(),
            hide_at_start,
            details_card: None,
            drag_out_tried: false,
            print: None,
            list_pick: None,
            glyph_px: mechanics::GLYPH_SIZE,
            confirm: None,
            native_menu: HashMap::new(),
            missing: missing::MissingState::new(),
            metrics_cache: HashMap::new(),
            logo: None,
            search_index: std::cell::OnceCell::new(),
            // Lets the automatic checks show the glyph filter and the search filled in.
            glyph_find: std::env::var("FONTCRAFT_GLYPH_FIND").unwrap_or_default(),
            glyph_kind: 0,
            watcher: None,
            watched: Vec::new(),
            reread_at: None,
            scanning: None,
            scan_again: false,
            icon_dark: None,
            icon_checked: -10.0,
            search: std::env::var("FONTCRAFT_SEARCH").unwrap_or_default(),
            preview: settings.default_preview.clone(),
            size: settings.default_size,
            selected,
            expanded: HashSet::new(),
            favorites,
            temp_active: HashSet::new(),
            // FONTCRAFT_UNFOLD (test pictures): every sidebar section open, without saving it.
            folded: if std::env::var("FONTCRAFT_UNFOLD").is_ok() { HashSet::new() } else { std::fs::read_to_string(folded_path()).map(|s| s.lines().filter(|l| !l.is_empty()).map(String::from).collect()).unwrap_or_default() },
            smart_extra: load_smart_extra(),
            cache: HashMap::new(),
            detail_tab: match std::env::var("FONTCRAFT_PANEL").as_deref() {
                Ok("sizes") => panel::DetailTab::Sizes,
                Ok("glyphs") => panel::DetailTab::Glyphs,
                Ok("features") => panel::DetailTab::Features,
                Ok("info") => panel::DetailTab::Info,
                _ => panel::DetailTab::Preview,
            },
            features: HashMap::new(),
            feature_open: std::env::var("FONTCRAFT_OPEN_FEATURES")
                .map(|v| v.split(',').map(String::from).collect())
                .unwrap_or_default(),
            glyph_sel: None,
            face_data: None,
            frame: 0,
            screenshot,
            shot_requested: false,
            shot_started: None,
            shot_asked_at: 0.0,
            shot_image: Default::default(),
            settings_open: std::env::var("FONT_SHELF_OPEN_SETTINGS").is_ok(),
            settings_tab: match std::env::var("FONT_SHELF_SETTINGS_TAB").as_deref() {
                Ok("general") => SettingsTab::General,
                Ok("library") => SettingsTab::Library,
                Ok("autoact") => SettingsTab::AutoActivation,
                _ => SettingsTab::Appearance,
            },
            pal: Pal::light(0),
            settings,
        };
        // A FontCraft library opened from the Finder / file manager (passed as an argument).
        let mut opened = false;
        for a in std::env::args().skip(1).filter(|a| a.to_lowercase().ends_with(".fcl")) {
            let p = std::fs::canonicalize(&a).unwrap_or_else(|_| PathBuf::from(&a));
            if p.is_dir() && !app.libraries.iter().any(|l| l.path == p) {
                let name = libscan::library_name(&p);
                app.libraries.push(libs::library_from(name, p, libs::LibKind::Fcl, true, &libscan::Preview::default()));
                app.view = View::Lib(app.libraries.len() - 1);
                opened = true;
            }
        }
        if opened {
            app.save_libraries();
            app.rescan();
        }
        app.compute_members();
        app.google_migrate();
        app.restore_open_state();
        if app.gf.open {
            gfonts::start_list(&app.gf.shared);
        }
        // Families downloaded earlier are already in the first reading (fonts::roots includes the
        // Google Fonts folder), so nothing is read again at start.
        if matches!(app.view, View::Lib(_) | View::LibSet(..)) {
            app.selected = app.rows().first().copied();
        }
        app.restore_active();
        if std::env::var("FONTCRAFT_PRINT").is_ok() {
            app.open_print();
        }
        // Lets the automatic checks show a switched-on font without saving anything.
        if let Ok(p) = std::env::var("FONTCRAFT_TEST_ACTIVE") {
            let p = PathBuf::from(p);
            if activate::set(&p, true) {
                app.active.insert(p);
            }
        }
        // The main window is known from the start (the menu bar asterisk may be clicked before
        // the window ever drew), and a background start keeps it see-through until it is put away.
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(h) = cc.window_handle() {
                if let RawWindowHandle::AppKit(a) = h.as_raw() {
                    app.ns_view = a.ns_view.as_ptr();
                    background::set_main_view(app.ns_view);
                }
            }
        }
        background::set_repaint_ctx(&cc.egui_ctx);
        if app.screenshot.is_none() {
            background::install();
        }
        if app.hide_at_start {
            background::see_through_until_put_away();
        }
        // FONTCRAFT_TEST_MENUBAR=<log file>: the menu bar trips, timed, on this Mac (a developer
        // check; FONTCRAFT_TEST_SWITCH_TO names the app the switch step brings forward).
        #[cfg(target_os = "macos")]
        if app.screenshot.is_none() {
            if let Ok(log) = std::env::var("FONTCRAFT_TEST_MENUBAR") {
                let probe = std::env::var("FONTCRAFT_TEST_SWITCH_TO").unwrap_or_else(|_| "com.apple.finder".into());
                background::menu_bar_selftest(PathBuf::from(log), probe);
            }
        }
        app
    }

    /// Back to the menu bar (the menu bar asterisk, the shortcut, or closing the window while
    /// FontCraft keeps running there). macOS: AppKit's own calls, at once (`background::hide_app`):
    /// a full-screen window first leaves full screen, see-through, and is put away when macOS says
    /// that is over. Elsewhere: the window library's commands.
    fn go_to_menu_bar(&mut self, ctx: &egui::Context) {
        background::set_in_background(true);
        if cfg!(target_os = "macos") {
            background::hide_app();
            return;
        }
        if ctx.input(|i| i.viewport().fullscreen).unwrap_or(false) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    /// Fonts added or removed in a library or folder outside FontCraft appear by themselves: the
    /// folders are watched (RightFont's RFFileWatcher, 0x10000c420) and read again a moment after the
    /// last change (so a big copy is read once, at the end).
    /// Keeps the app icon (macOS Dock, the window's icon elsewhere) in step with the system's light
    /// or dark look, checked every 2 seconds.
    fn follow_icon_look(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        // Back in the Dock from the menu bar: a new Dock tile, so the icon is set again at once.
        let due = background::take_dock_icon_due();
        if self.screenshot.is_some() || (!due && now - self.icon_checked < 2.0) {
            return;
        }
        self.icon_checked = now;
        let dark = match ctx.system_theme() {
            Some(t) => t == egui::Theme::Dark,
            None => appicon::system_is_dark(),
        };
        if self.icon_dark == Some(dark) && !due {
            return;
        }
        self.icon_dark = Some(dark);
        #[cfg(target_os = "macos")]
        appicon::set_dock_icon_styled(dark, self.settings.widget_style, self.settings.accent);
        #[cfg(not(target_os = "macos"))]
        ctx.send_viewport_cmd(egui::ViewportCommand::Icon(Some(std::sync::Arc::new(appicon::window_icon_styled(dark, self.settings.widget_style, self.settings.accent)))));
    }

    fn watch_libraries(&mut self, ctx: &egui::Context) {
        self.follow_icon_look(ctx);
        self.finish_rescan(ctx);
        if self.screenshot.is_some() {
            return;
        }
        let paths = self.library_paths();
        if paths != self.watched {
            self.watcher = if paths.is_empty() { None } else { watch::Watcher::new(&paths) };
            self.watched = paths;
        }
        let now = ctx.input(|i| i.time);
        if self.watcher.as_ref().is_some_and(|w| w.take_changed()) {
            self.reread_at = Some(now + watch::DEBOUNCE_S);
        }
        match self.reread_at {
            Some(t) if now >= t => {
                self.reread_at = None;
                for l in self.libraries.iter_mut() {
                    l.sets = libs::sets_for(&l.path, l.kind);
                }
                self.rescan_soon();
            }
            Some(_) => ctx.request_repaint_after(std::time::Duration::from_millis(250)),
            None => ctx.request_repaint_after(std::time::Duration::from_millis(1000)),
        }
    }

    /// Opens or closes a family's styles in the list, and remembers it (RightFont keeps its open
    /// families under ExpandedFontFamilies, key "<library>-<family>", 0x1001081a8).
    fn toggle_expanded(&mut self, family: usize) {
        if !self.expanded.remove(&family) {
            self.expanded.insert(family);
        }
        let mut names: Vec<String> = self.expanded.iter().filter_map(|&f| self.lib.families.get(f)).map(|f| f.name.clone()).collect();
        names.sort();
        self.settings.expanded = names;
        self.settings.save();
    }

    /// Folds or unfolds a library in the sidebar, and remembers it.
    pub(crate) fn toggle_collapsed(&mut self, lib: usize) {
        if !self.lib_collapsed.remove(&lib) {
            self.lib_collapsed.insert(lib);
        }
        let mut paths: Vec<String> = self.lib_collapsed.iter().filter_map(|&i| self.libraries.get(i)).map(|l| l.path.display().to_string()).collect();
        paths.sort();
        self.settings.collapsed = paths;
        self.settings.save();
    }

    /// After the fonts or libraries are read again (and at start): open the families and fold the
    /// libraries that were open/folded before, found by name and path (RightFont's
    /// -[RFFontListViewController restoreExpandedState], 0x1001082a0, does this after each reload).
    pub(crate) fn restore_open_state(&mut self) {
        let names: HashSet<&str> = self.settings.expanded.iter().map(String::as_str).collect();
        self.expanded = (0..self.lib.families.len()).filter(|&i| names.contains(self.lib.families[i].name.as_str())).collect();
        let paths: HashSet<String> = self.settings.collapsed.iter().cloned().collect();
        self.lib_collapsed = (0..self.libraries.len()).filter(|&i| paths.contains(&self.libraries[i].path.display().to_string())).collect();
    }

    /// Settings › General › Apply: shares ~/FontCraft as chosen, right away.
    fn apply_share(&mut self) {
        let lang = self.settings.lang;
        let t = move |k: K| i18n::tr(lang, k);
        let with = if self.share_all == Some(true) { fcl::ShareWith::Everyone } else { fcl::ShareWith::User(self.share_user.clone()) };
        let before = fcl::ShareWith::from_setting(&self.settings.share);
        let home = paths::home();
        match fcl::share(&fcl::home_dir(), &with, before.as_ref(), Some(&home)) {
            Ok(()) => {
                self.settings.share = with.to_setting();
                self.settings.save();
                self.share_status = Some((
                    true,
                    match &with {
                        fcl::ShareWith::Everyone => format!("✓ {}", t(K::AllUsers)),
                        fcl::ShareWith::User(u) => t(K::SharedWith).replace("{who}", u),
                    },
                ));
            }
            Err(e) => {
                // A folder the system protects: open its settings so FontCraft can be allowed.
                if e.contains("not permitted") {
                    fcl::open_security(fcl::Pane::Files);
                }
                self.share_status = Some((false, t(K::ShareFailed).replace("{err}", &e)));
            }
        }
    }

    fn save_favorites(&self) {
        let p = favorites_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut v: Vec<_> = self.favorites.iter().cloned().collect();
        v.sort();
        let _ = std::fs::write(p, v.join("\n"));
    }

    pub(crate) fn save_smart_extra(&self) {
        let p = smart_extra_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut v: Vec<String> = self.smart_extra.iter().flat_map(|(k, fams)| fams.iter().map(move |f| format!("{k}\t{f}"))).collect();
        v.sort();
        let _ = std::fs::write(p, v.join("\n"));
    }

    /// The smart lists, in the sidebar's order, with their names.
    pub(crate) fn smart_lists(&self) -> Vec<(View, &'static str)> {
        let t = |k: K| i18n::tr(self.settings.lang, k);
        vec![
            (View::Class(fonts::Class::Sans), t(K::SansList)),
            (View::Class(fonts::Class::Serif), t(K::SerifList)),
            (View::Class(fonts::Class::Slab), t(K::SlabList)),
            (View::Class(fonts::Class::Script), t(K::ScriptList)),
            (View::Class(fonts::Class::Blackletter), t(K::BlackletterList)),
            (View::Class(fonts::Class::Display), t(K::CatDisplay)),
            (View::Class(fonts::Class::Symbol), t(K::SymbolList)),
            (View::Mono, t(K::Monospaced)),
        ]
    }

    /// Whether family `i` is in a smart list: by its kind, or added to it by hand.
    pub(crate) fn in_smart_list(&self, i: usize, view: View) -> bool {
        let f = &self.lib.families[i];
        let by_kind = match view {
            View::Mono => f.monospaced,
            View::Class(c) => f.class == c,
            _ => false,
        };
        by_kind || menus::smart_key(view).and_then(|k| self.smart_extra.get(k)).is_some_and(|s| s.contains(&f.name))
    }

    fn family_matches(&self, i: usize, view: View) -> bool {
        let f = &self.lib.families[i];
        if !self.settings.source_enabled(f.source) {
            return false;
        }
        let in_view = match view {
            View::All => true,
            View::Favorites => self.favorites.contains(&f.name),
            View::Source(s) => f.source == s,
            View::Mono | View::Class(_) => self.in_smart_list(i, view),
            View::Lib(l) => f.libs.contains(&l),
            View::LibSet(l, s) => self.members.get(l).and_then(|m| m.sets.get(s)).is_some_and(|set| set.contains(&i)),
        };
        in_view && (self.search.is_empty() || self.search_text(i).contains(&self.search.to_lowercase()))
    }

    /// What the search box looks through for one family, like RightFont's search (query at
    /// 0x100285208: name, full name, display name, family group, style, manufacturer, designer,
    /// copyright, license — each "LIKE '%text%'"). Built on the first search, kept until fonts are read again.
    fn search_text(&self, family: usize) -> &str {
        let index = self.search_index.get_or_init(|| {
            use ttf_parser::name_id;
            self.lib
                .families
                .iter()
                .map(|fam| {
                    let mut text = fam.name.to_lowercase();
                    for &face in &fam.faces {
                        let fc = &self.lib.faces[face];
                        text.push('\n');
                        text.push_str(&fc.style.to_lowercase());
                        let wanted: &[u16] = if face == fam.default_face {
                            &[name_id::FULL_NAME, name_id::TYPOGRAPHIC_FAMILY, name_id::MANUFACTURER, name_id::DESIGNER, name_id::COPYRIGHT_NOTICE, name_id::LICENSE]
                        } else {
                            &[name_id::FULL_NAME]
                        };
                        if let Ok(tf) = ttf_parser::Face::parse(&self.lib.files[fc.file].data, fc.index) {
                            for id in wanted {
                                if let Some(v) = tf.names().into_iter().filter(|n| n.name_id == *id).find_map(|n| n.to_string()) {
                                    text.push('\n');
                                    text.push_str(&v.to_lowercase());
                                }
                            }
                        }
                    }
                    text
                })
                .collect()
        });
        index.get(family).map(String::as_str).unwrap_or("")
    }

    fn count(&self, view: View) -> usize {
        // A library's list only needs to look at its own families (a folder can hold a thousand
        // lists; checking every family for each of them on every frame made the window slow).
        if let View::LibSet(l, s) = view {
            return self.members.get(l).and_then(|m| m.sets.get(s)).map_or(0, |set| set.iter().filter(|&&i| self.family_matches(i, view)).count());
        }
        (0..self.lib.families.len()).filter(|&i| self.family_matches(i, view)).count()
    }

    fn rows(&self) -> Vec<Row> {
        let mut fams: Vec<usize> = (0..self.lib.families.len()).filter(|&i| self.family_matches(i, self.view)).collect();
        // RightFont lists a library's fonts by name like everywhere else (its lists, not its fonts,
        // follow the library's own creation order), so families stay in name order here too.
        let _ = &mut fams;
        let mut out = Vec::new();
        for i in fams {
            out.push(Row::Family(i));
            if self.expanded.contains(&i) {
                for &face in &self.lib.families[i].faces {
                    out.push(Row::Face(i, face));
                }
            }
        }
        out
    }

    /// Switched-on features, in the order they're applied.
    fn active_features(&self) -> Vec<[u8; 4]> {
        let mut tags: Vec<[u8; 4]> = ot::MAIN_FEATURES
            .iter()
            .map(|t| **t)
            .filter(|t| *self.features.get(t).unwrap_or(&ot::on_by_default(t)))
            .collect();
        for n in 1..=20u8 {
            let t = ot::ss_tag(n);
            if *self.features.get(&t).unwrap_or(&false) {
                tags.push(t);
            }
        }
        tags.sort_by_key(|t| ot::apply_order(t));
        tags
    }

    fn feature_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.active_features().hash(&mut h);
        h.finish()
    }

    /// Draws (once) and caches a picture made by `make`, which gets the font bytes, face index and pixel size.
    fn cached(
        &mut self,
        ctx: &egui::Context,
        key: TexKey,
        px: f32,
        make: impl FnOnce(&[u8], u32, f32) -> Option<fonts::Rendered>,
    ) -> (Option<TextureHandle>, Vec2, bool, bool) {
        let ppp = ctx.pixels_per_point();
        let frame = self.frame;
        if let Some(c) = self.cache.get_mut(&key) {
            c.last = frame;
            return (c.tex.clone(), c.size, c.drew, c.missing);
        }
        let face = key.1;
        let f = &self.lib.faces[face];
        let data = &self.lib.files[f.file].data;
        let r = make(data, f.index, px * ppp);
        let entry = match r {
            Some(r) if r.drew => {
                let pixels = r.alpha.iter().map(|&a| Color32::from_white_alpha(a)).collect();
                let img = ColorImage::new([r.w, r.h], pixels);
                let tex = ctx.load_texture(format!("p{face}-{frame}"), img, TextureOptions::LINEAR);
                Cached { tex: Some(tex), size: Vec2::new(r.w as f32 / ppp, r.h as f32 / ppp), drew: true, missing: r.missing, last: frame }
            }
            Some(r) => Cached { tex: None, size: Vec2::new(r.w as f32 / ppp, r.h as f32 / ppp), drew: false, missing: r.missing, last: frame },
            None => Cached { tex: None, size: Vec2::ZERO, drew: false, missing: false, last: frame },
        };
        let out = (entry.tex.clone(), entry.size, entry.drew, entry.missing);
        self.cache.insert(key, entry);
        out
    }

    /// The preview text drawn in a given font on one line, cached as a texture.
    fn texture(&mut self, ctx: &egui::Context, face: usize, text: &str, px: f32, max_w: f32) -> (Option<TextureHandle>, Vec2, bool, bool) {
        let ppp = ctx.pixels_per_point();
        let max_px = ((max_w * ppp) as u32 / 64 + 1) * 64;
        let feats = self.active_features();
        let key = (0, face, text.to_string(), (px * ppp).round() as u32, max_px, self.feature_hash());
        self.cached(ctx, key, px, |d, i, px| fonts::render(d, i, text, px, max_px as usize, &feats, false))
    }

    /// Like `texture`, but long text wraps onto more lines within `max_w`.
    fn texture_wrapped(&mut self, ctx: &egui::Context, face: usize, text: &str, px: f32, max_w: f32) -> (Option<TextureHandle>, Vec2, bool, bool) {
        let ppp = ctx.pixels_per_point();
        let max_px = ((max_w * ppp) as u32 / 8 * 8).max(8);
        let feats = self.active_features();
        let key = (1, face, text.to_string(), (px * ppp).round() as u32, max_px, self.feature_hash());
        self.cached(ctx, key, px, |d, i, px| fonts::render(d, i, text, px, max_px as usize, &feats, true))
    }

    /// A row of glyphs by number, exactly as given (feature examples).
    fn texture_ids(&mut self, ctx: &egui::Context, face: usize, ids: &[u16], px: f32) -> (Option<TextureHandle>, Vec2, bool, bool) {
        let ppp = ctx.pixels_per_point();
        let text: String = ids.iter().map(|g| format!("{g},")).collect();
        let key = (2, face, text, (px * ppp).round() as u32, 0, 0);
        self.cached(ctx, key, px, |d, i, px| fonts::render_ids(d, i, ids, px, 4000))
    }

    /// One glyph, centred (Glyphs page).
    fn texture_glyph(&mut self, ctx: &egui::Context, face: usize, gid: u16, px: f32) -> (Option<TextureHandle>, Vec2, bool, bool) {
        let ppp = ctx.pixels_per_point();
        let key = (3, face, String::new(), (px * ppp).round() as u32, gid as u32, 0);
        self.cached(ctx, key, px, |d, i, px| fonts::render_glyph(d, i, gid, px))
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let pref = self.settings.theme_preference();
        if ctx.options(|o| o.theme_preference) != pref {
            ctx.set_theme(pref);
        }
        let dark = ctx.theme() == egui::Theme::Dark;
        let acc = self.settings.accent;
        let pal = if dark { Pal::dark(acc) } else { Pal::light(acc) };
        if pal != self.pal || self.frame == 1 {
            self.pal = pal;
            ctx.set_visuals_of(egui::Theme::Light, Pal::light(acc).visuals());
            ctx.set_visuals_of(egui::Theme::Dark, Pal::dark(acc).visuals());
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let pal = self.pal;
        let lang = self.settings.lang;
        let t = move |k: K| i18n::tr(lang, k);
        let before = self.settings.clone();
        let mut rescan = false;
        let mut clear_favs = false;
        let mut apply_share = false;
        // Its own system window, as tall as what the chosen tab shows.
        let (_, closed) = oslook::dialog(ctx, "settings", t(K::Settings), 680.0, pal.panel, None, |ui| {
                // Tab bar: icon above label, like a Mac settings window.
                ui.horizontal(|ui| {
                    let tabs = [
                        (SettingsTab::General, t(K::General)),
                        (SettingsTab::Appearance, t(K::Appearance)),
                        (SettingsTab::Library, t(K::LibraryTab)),
                        (SettingsTab::AutoActivation, t(K::AutoActivationTab)),
                    ];
                    let tab_w = 112.0;
                    ui.add_space((ui.available_width() - tab_w * tabs.len() as f32 - 24.0).max(0.0) / 2.0);
                    for (tab, label) in tabs {
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(tab_w, 54.0), Sense::click());
                        let on = self.settings_tab == tab;
                        if on {
                            ui.painter().rect_filled(r, 8.0, pal.accent_soft);
                        } else if resp.hovered() {
                            ui.painter().rect_filled(r, 8.0, pal.hover);
                        }
                        let col = if on { pal.accent } else { pal.muted };
                        let ic = r.center_top() + Vec2::new(0.0, 19.0);
                        let p = ui.painter();
                        match tab {
                            SettingsTab::General => {
                                p.text(ic, Align2::CENTER_CENTER, "Aa", FontId::proportional(18.0), col);
                            }
                            SettingsTab::Appearance => {
                                p.circle_stroke(ic, 8.0, Stroke::new(1.6_f32, col));
                                let mut pts = vec![];
                                for i in 0..=16 {
                                    let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 16.0;
                                    pts.push(ic + Vec2::angled(a) * 8.0);
                                }
                                p.add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
                            }
                            // The sidebar's own library icon, and the automation icon (SVGs in assets/icons/ui).
                            SettingsTab::Library => uiicons::paint(ui, "library", ic, 22.0, col),
                            SettingsTab::AutoActivation => uiicons::paint(ui, "automation", ic, 22.0, col),
                        }
                        // Long tab names shrink to fit.
                        let galley = p.layout_no_wrap(label.to_string(), FontId::proportional(12.5), pal.ink);
                        let size = if galley.size().x > tab_w - 8.0 { 12.5 * (tab_w - 8.0) / galley.size().x } else { 12.5 };
                        p.text(r.center_bottom() - Vec2::new(0.0, 11.0), Align2::CENTER_CENTER, label, FontId::proportional(size), if on { pal.accent } else { pal.ink });
                        if resp.clicked() {
                            self.settings_tab = tab;
                        }
                    }
                });
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(8.0);
                let heading = |ui: &mut egui::Ui, t: &str| {
                    ui.label(egui::RichText::new(t).size(13.5).strong().color(pal.ink));
                };
                let note = |ui: &mut egui::Ui, t: &str| {
                    ui.add(egui::Label::new(egui::RichText::new(t).size(12.0).color(pal.muted)).wrap());
                };
                match self.settings_tab {
                    SettingsTab::AutoActivation => self.autoact_settings(ui),
                    SettingsTab::General => {
                        // Start-up: open at login, keep running in the menu bar, and a shortcut to open.
                        egui::Grid::new("startup").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
                            heading(ui, t(K::StartupHead));
                            ui.vertical(|ui| {
                                oslook::check(ui, &mut self.settings.launch_at_login, t(K::LaunchAtLogin));
                                oslook::check(ui, &mut self.settings.background, t(K::RunInBackground));
                                note(ui, t(K::BackgroundNote));
                            });
                            ui.end_row();
                            heading(ui, t(K::OpenShortcut));
                            ui.horizontal(|ui| {
                                let shown = if self.recording_hotkey {
                                    t(K::PressKeys).to_string()
                                } else if self.settings.hotkey.is_empty() {
                                    t(K::NoShortcut).to_string()
                                } else {
                                    background::pretty_hotkey(&self.settings.hotkey)
                                };
                                egui::Frame::NONE.fill(pal.card).corner_radius(6).inner_margin(Margin::symmetric(10, 5)).show(ui, |ui| {
                                    ui.label(egui::RichText::new(shown).monospace().color(if self.recording_hotkey { pal.accent } else { pal.ink }));
                                });
                                if ui.button(t(K::RecordShortcut)).clicked() {
                                    self.recording_hotkey = true;
                                }
                                if !self.settings.hotkey.is_empty() && ui.button(t(K::ClearShortcut)).clicked() {
                                    self.settings.hotkey.clear();
                                    self.recording_hotkey = false;
                                }
                            });
                            ui.end_row();
                        });
                        if self.recording_hotkey {
                            if let Some(hk) = background::read_hotkey(ui.ctx()) {
                                self.settings.hotkey = hk;
                                self.recording_hotkey = false;
                            }
                        }
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);
                        // Share ~/FontCraft (FontCraft's libraries) with everyone or one user; Apply makes it so now.
                        egui::Grid::new("share").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
                            heading(ui, t(K::ShareWith));
                            ui.vertical(|ui| {
                                let users = self.users.get_or_insert_with(fcl::other_users).clone();
                                oslook::radio(ui, &mut self.share_all, Some(true), t(K::AllUsers));
                                ui.horizontal(|ui| {
                                    oslook::radio(ui, &mut self.share_all, Some(false), t(K::User));
                                    if users.is_empty() {
                                        ui.label(egui::RichText::new(t(K::NoOtherUsers)).size(12.5).color(pal.muted));
                                    } else {
                                        if !users.contains(&self.share_user) {
                                            self.share_user = users[0].clone();
                                        }
                                        let was = self.share_user.clone();
                                        oslook::combo("share-user", self.share_user.clone()).width(200.0).show_ui(ui, |ui| {
                                            for u in &users {
                                                ui.selectable_value(&mut self.share_user, u.clone(), u);
                                            }
                                        });
                                        if self.share_user != was {
                                            self.share_all = Some(false);
                                        }
                                    }
                                });
                                let can = self.share_all == Some(true) || (self.share_all == Some(false) && !users.is_empty());
                                if oslook::default_button(ui, can, t(K::ApplyBtn)).clicked() {
                                    apply_share = true;
                                }
                                if let Some((ok, msg)) = &self.share_status {
                                    ui.label(egui::RichText::new(msg).size(12.5).color(if *ok { pal.accent } else { pal.warn }));
                                }
                                note(ui, &t(K::ShareNote).replace("{dir}", &fcl::home_dir().display().to_string()));
                            });
                            ui.end_row();
                            // What FontCraft needs from the system, with a way to the system's own settings.
                            heading(ui, t(K::PermissionsHead));
                            ui.vertical(|ui| {
                                let ok = autoact::trusted();
                                ui.label(egui::RichText::new(if ok { t(K::AccessAllowed) } else { t(K::AccessNotAllowed) }).color(if ok { pal.ink } else { pal.warn }));
                                ui.horizontal(|ui| {
                                    if ui.button(t(K::OpenSecurity)).clicked() {
                                        fcl::open_security(fcl::Pane::Accessibility);
                                    }
                                    if ui.button(t(K::AllowAutomation)).clicked() {
                                        fcl::open_security(fcl::Pane::Automation);
                                    }
                                });
                            });
                            ui.end_row();
                        });
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);
                        egui::Grid::new("general").num_columns(2).spacing([16.0, 12.0]).show(ui, |ui| {
                            heading(ui, t(K::Language));
                            ui.vertical(|ui| {
                                oslook::combo("language", i18n::LANGS[self.settings.lang].name)
                                    .width(240.0)
                                    .height(320.0)
                                    .show_ui(ui, |ui| {
                                        for (i, l) in i18n::LANGS.iter().enumerate() {
                                            ui.selectable_value(&mut self.settings.lang, i, l.name);
                                        }
                                    });
                                note(ui, t(K::LanguageNote));
                            });
                            ui.end_row();
                            heading(ui, t(K::PreviewText));
                            ui.vertical(|ui| {
                                oslook::field(ui, egui::TextEdit::singleline(&mut self.settings.default_preview).desired_width(320.0).margin(Margin::symmetric(8, 5)));
                                if ui.small_button(t(K::UseCurrent)).clicked() {
                                    self.settings.default_preview = self.preview.clone();
                                }
                            });
                            ui.end_row();
                            heading(ui, t(K::PreviewSize));
                            ui.horizontal(|ui| {
                                ui.spacing_mut().slider_width = 220.0;
                                ui.add(egui::Slider::new(&mut self.settings.default_size, 10.0..=160.0).suffix(" px").integer());
                            });
                            ui.end_row();
                            heading(ui, t(K::FontList));
                            ui.vertical(|ui| {
                                oslook::check(ui, &mut self.settings.show_style_count, t(K::ShowStyleCount));
                                oslook::check(ui, &mut self.settings.warn_missing, t(K::WarnMissing));
                            });
                            ui.end_row();
                            heading(ui, t(K::DetailsPanel));
                            ui.vertical(|ui| {
                                oslook::check(ui, &mut self.settings.show_details, t(K::ShowDetails));
                            });
                            ui.end_row();
                        });
                        ui.add_space(8.0);
                        note(ui, t(K::GeneralNote));
                    }
                    SettingsTab::Appearance => {
                        heading(ui, t(K::Theme));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            let acc = self.settings.accent;
                            for (choice, label) in [(ThemeChoice::System, t(K::MatchSystem)), (ThemeChoice::Light, t(K::Light)), (ThemeChoice::Dark, t(K::Dark))] {
                                let (r, resp) = ui.allocate_exact_size(Vec2::new(150.0, 112.0), Sense::click());
                                let on = self.settings.theme == choice;
                                let win = Rect::from_min_size(r.min + Vec2::new(8.0, 6.0), Vec2::new(134.0, 80.0));
                                let p = ui.painter();
                                match choice {
                                    ThemeChoice::Light => settings::paint_mini_window(p, win, &Pal::light(acc)),
                                    ThemeChoice::Dark => settings::paint_mini_window(p, win, &Pal::dark(acc)),
                                    ThemeChoice::System => {
                                        let half = Rect::from_min_max(win.min, Pos2::new(win.center().x, win.bottom()));
                                        settings::paint_mini_window(&p.with_clip_rect(half), win, &Pal::light(acc));
                                        let other = Rect::from_min_max(Pos2::new(win.center().x, win.top()), win.max);
                                        settings::paint_mini_window(&p.with_clip_rect(other), win, &Pal::dark(acc));
                                    }
                                }
                                if on {
                                    p.rect_stroke(win.expand(3.0), 8.0, Stroke::new(2.5_f32, pal.accent), eframe::egui::StrokeKind::Inside);
                                } else if resp.hovered() {
                                    p.rect_stroke(win.expand(3.0), 8.0, Stroke::new(1.5_f32, pal.muted), eframe::egui::StrokeKind::Inside);
                                }
                                p.text(r.center_bottom() - Vec2::new(0.0, 12.0), Align2::CENTER_CENTER, label, FontId::proportional(13.0), if on { pal.accent } else { pal.ink });
                                if resp.clicked() {
                                    self.settings.theme = choice;
                                }
                            }
                        });
                        ui.add_space(12.0);
                        egui::Grid::new("appearance").num_columns(2).spacing([16.0, 14.0]).show(ui, |ui| {
                            let is_es = crate::i18n::LANGS[self.settings.lang].code == "es";
                            heading(ui, if is_es { "Tema" } else { t(K::Accent) });
                            ui.horizontal(|ui| {
                                // 1. Multicolor (Rainbow swatch matching macOS System Settings)
                                {
                                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                                    settings::paint_multicolor_swatch(ui.painter(), r.center(), 9.0);
                                    if self.settings.accent == settings::MULTICOLOR {
                                        ui.painter().circle_stroke(r.center(), 11.5, Stroke::new(2.0_f32, pal.accent));
                                        ui.painter().circle_filled(r.center(), 3.0, Color32::WHITE);
                                    }
                                    if resp.on_hover_text("Multicolor").clicked() {
                                        self.settings.accent = settings::MULTICOLOR;
                                        self.logo = None;
                                    }
                                    ui.add_space(4.0);
                                }
                                // 2. The 8 macOS theme colors (Azul, Púrpura, Rosa, Rojo, Naranja, Amarillo, Verde, Grafito)
                                for (i, (name, light, dark)) in settings::ACCENTS.iter().enumerate() {
                                    let col = if pal.dark { *dark } else { *light };
                                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                                    ui.painter().circle_filled(r.center(), 9.0, col);
                                    if self.settings.accent == i + 1 {
                                        ui.painter().circle_stroke(r.center(), 11.5, Stroke::new(2.0_f32, col));
                                        ui.painter().circle_filled(r.center(), 3.0, Color32::WHITE);
                                    }
                                    let localized_name = match (is_es, i) {
                                        (false, 0) => "Blue",
                                        (false, 1) => "Purple",
                                        (false, 2) => "Pink",
                                        (false, 3) => "Red",
                                        (false, 4) => "Orange",
                                        (false, 5) => "Yellow",
                                        (false, 6) => "Green",
                                        (false, 7) => "Graphite",
                                        _ => *name,
                                    };
                                    if resp.on_hover_text(localized_name).clicked() {
                                        self.settings.accent = i + 1;
                                        self.logo = None;
                                    }
                                }
                            });
                            ui.end_row();

                            // Estilo de iconos y widgets (macOS Sequoia / HIG)
                            heading(ui, if is_es { "Estilo de iconos y widgets" } else { "Icon & widget style" });
                            ui.horizontal(|ui| {
                                for (st, label) in [
                                    (settings::WidgetStyle::Default, if is_es { "Predeterminado" } else { "Default" }),
                                    (settings::WidgetStyle::Dark, if is_es { "Oscuro" } else { "Dark" }),
                                    (settings::WidgetStyle::Clear, if is_es { "Transparente" } else { "Clear" }),
                                    (settings::WidgetStyle::Tinted, if is_es { "Teñido" } else { "Tinted" }),
                                ] {
                                    if ui.selectable_label(self.settings.widget_style == st, label).clicked() {
                                        self.settings.widget_style = st;
                                        self.logo = None;
                                    }
                                }
                            });
                            ui.end_row();

                            // Botones radiales de confirmación ("Siempre" y "Automático")
                            heading(ui, if is_es { "Confirmación" } else { "Confirmation" });
                            ui.horizontal(|ui| {
                                oslook::radio(ui, &mut self.settings.confirm_mode, settings::ConfirmMode::Auto, if is_es { "Automático" } else { "Automatic" });
                                ui.add_space(14.0);
                                oslook::radio(ui, &mut self.settings.confirm_mode, settings::ConfirmMode::Always, if is_es { "Siempre" } else { "Always" });
                            });
                            ui.end_row();

                            heading(ui, t(K::RowSpacing));
                            ui.horizontal(|ui| {
                                for (i, name) in [t(K::Compact), t(K::Regular), t(K::Roomy)].iter().enumerate() {
                                    if ui.selectable_label(self.settings.density == i, *name).clicked() {
                                        self.settings.density = i;
                                    }
                                }
                            });
                            ui.end_row();
                        });
                    }
                    SettingsTab::Library => {
                        heading(ui, t(K::ShowFontsFrom));
                        ui.add_space(4.0);
                        // The first existing font folder of each kind on this system (macOS, Windows, Linux).
                        let roots = paths::font_roots();
                        let folder = |k: u8| {
                            roots.iter().filter(|(_, kk)| *kk == k).find(|(p, _)| p.exists()).or_else(|| roots.iter().find(|(_, kk)| *kk == k)).map(|(p, _)| p.display().to_string()).unwrap_or_default()
                        };
                        let rows = [
                            (0, Source::System, t(K::SystemFonts), folder(0)),
                            (1, Source::Computer, t(K::ComputerFonts), folder(1)),
                            (2, Source::User, t(K::YourFonts), folder(2)),
                        ];
                        for (i, src, label, path) in rows {
                            let n = self.lib.families.iter().filter(|f| f.source == src).count();
                            ui.horizontal(|ui| {
                                oslook::check(ui, &mut self.settings.sources[i], label);
                                ui.label(egui::RichText::new(if n == 1 { t(K::Family1).to_string() } else { t(K::FamiliesN).replace("{n}", &n.to_string()) }).size(12.0).color(pal.muted));
                            });
                            ui.horizontal(|ui| {
                                ui.add_space(28.0);
                                note(ui, &path);
                            });
                        }
                        // Google Fonts and Adobe Fonts are sources too (not libraries).
                        let n_google = self.lib.families.iter().filter(|f| f.source == Source::Google).count();
                        let n_adobe = self.lib.families.iter().filter(|f| f.source == Source::Adobe).count();
                        for (flag, label, n) in [
                            (&mut self.settings.show_google, t(K::ShowGoogle), n_google),
                            (&mut self.settings.show_adobe, t(K::ShowAdobe), n_adobe),
                        ] {
                            ui.horizontal(|ui| {
                                oslook::check(ui, flag, label);
                                ui.label(egui::RichText::new(if n == 1 { t(K::Family1).to_string() } else { t(K::FamiliesN).replace("{n}", &n.to_string()) }).size(12.0).color(pal.muted));
                            });
                        }
                        ui.horizontal(|ui| {
                            ui.add_space(28.0);
                            note(ui, t(K::AdobeHint));
                        });
                        // RightFont's ShowStarredFonts / ShowSmartFilters (on by default).
                        ui.add_space(6.0);
                        let smart = {
                            let low = t(K::SmartListsHead).to_lowercase();
                            let mut c = low.chars();
                            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
                        };
                        oslook::check(ui, &mut self.settings.show_favorites, t(K::Favorites));
                        oslook::check(ui, &mut self.settings.show_smart, smart);
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            // "Look for new fonts" opens the Import Fonts window (into one of the libraries).
                            if ui.button(t(K::ImportFonts)).clicked() {
                                self.import_dlg = Some(importdlg::ImportDlg::default());
                            }
                            if false {
                                rescan = true;
                            }
                            let n = self.favorites.len();
                            if ui.add_enabled(n > 0, egui::Button::new(t(K::ClearFavs).replace("{n}", &n.to_string()))).clicked() {
                                clear_favs = true;
                            }
                        });
                        ui.add_space(10.0);
                        note(ui, t(K::ReadOnlyNote));
                    }
                }
            });
        if closed {
            self.settings_open = false;
        }
        if apply_share {
            self.apply_share();
        }
        if rescan {
            self.rescan();
            self.cache.clear();
            self.selected = if self.lib.families.is_empty() { None } else { Some(Row::Family(0)) };
        }
        if clear_favs {
            self.favorites.clear();
            self.save_favorites();
        }
        if self.settings != before {
            if self.settings.launch_at_login != before.launch_at_login {
                background::set_login_item(self.settings.launch_at_login);
            }
            if self.settings.show_google != before.show_google || self.settings.show_adobe != before.show_adobe {
                self.cache.clear();
            }
            if self.settings.default_size != before.default_size {
                self.size = self.settings.default_size;
            }
            if self.settings.lang != before.lang {
                // Swap the sample sentence for the new language's one, unless the person typed their own.
                let new = i18n::tr(self.settings.lang, K::Pangram);
                if i18n::is_sample(&self.preview) {
                    self.preview = new.to_string();
                }
                if i18n::is_sample(&self.settings.default_preview) {
                    self.settings.default_preview = new.to_string();
                }
            } else if self.settings.default_preview != before.default_preview {
                self.preview = self.settings.default_preview.clone();
            }
            self.settings.save();
        }
    }

    fn preview_text(&self, family: usize) -> String {
        if self.preview.trim().is_empty() {
            self.lib.families[family].name.clone()
        } else {
            self.preview.clone()
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let lang = self.settings.lang;
        let t = move |k: K| i18n::tr(lang, k);
        // Finder's sidebar: a little room above the first section, inside the card.
        ui.add_space(6.0);
        // Google Fonts and Adobe Fonts are sources (shown when switched on in Settings › Library).
        let mut sources = vec![
            (View::Source(Source::System), t(K::System)),
            (View::Source(Source::Computer), t(K::Computer)),
            (View::Source(Source::User), t(K::User)),
        ];
        if self.settings.show_google {
            sources.push((View::Source(Source::Google), t(K::GoogleFonts)));
        }
        if self.settings.show_adobe {
            sources.push((View::Source(Source::Adobe), t(K::AdobeFonts)));
        }
        let mut library = vec![(View::All, t(K::AllFonts))];
        if self.settings.show_favorites {
            library.push((View::Favorites, t(K::Favorites)));
        }
        let mut sections: Vec<(&str, Vec<(View, &str)>)> = vec![(t(K::LibraryHead), library), (t(K::SourcesHead), sources)];
        if self.settings.show_smart {
            sections.push((
                t(K::SmartListsHead),
                vec![
                    // RightFont's order (its classes 1–9, 0x10030f048): sans serif, serif, slab serif,
                    // script, blackletter, display, symbols, monospaced.
                    (View::Class(fonts::Class::Sans), t(K::SansList)),
                    (View::Class(fonts::Class::Serif), t(K::SerifList)),
                    (View::Class(fonts::Class::Slab), t(K::SlabList)),
                    (View::Class(fonts::Class::Script), t(K::ScriptList)),
                    (View::Class(fonts::Class::Blackletter), t(K::BlackletterList)),
                    (View::Class(fonts::Class::Display), t(K::CatDisplay)),
                    (View::Class(fonts::Class::Symbol), t(K::SymbolList)),
                    (View::Mono, t(K::Monospaced)),
                ],
            ));
        }
        for (n, (title, items)) in sections.into_iter().enumerate() {
            // Finder's section title: grey, folds the section away, chevron at the right on hover.
            let key = ["library", "sources", "smart"].get(n).copied().unwrap_or("other");
            let open = !self.folded.contains(key);
            if finder::section_header(ui, &pal, title, open, |_| {}) {
                self.toggle_folded(key);
            }
            if !open {
                ui.add_space(2.0);
                continue;
            }
            for (view, label) in items {
                let count = self.count(view);
                let sel = self.view == view;
                let (resp, r, _) = finder::row(
                    ui,
                    &pal,
                    finder::Row { icon: finder::Icon::Symbol(view_symbol(view)), label, count: Some(count), selected: sel, indent: 0.0, disclosure: None },
                    |p, c, icol| paint_view_icon(p, view, c, icol),
                );
                // Google Fonts: a + to browse and download; Adobe Fonts: an arrow to fonts.adobe.com.
                // Both show under the pointer, left of the count, as Finder shows a row's actions.
                let service = matches!(view, View::Source(Source::Google) | View::Source(Source::Adobe));
                let count_w = { let m = finder::metrics(); finder::text_width(ui, &count.to_string(), m.count, 0.0) };
                let btn = Rect::from_center_size(Pos2::new(r.right() - 8.0 - count_w - 16.0, r.center().y), Vec2::splat(18.0));
                if service && (resp.hovered() || sel) {
                    let hot = ui.rect_contains_pointer(btn);
                    let bc = if hot { pal.accent } else { pal.muted };
                    if hot {
                        ui.painter().rect_filled(btn, 5.0, pal.hover);
                    }
                    let sym = if view == View::Source(Source::Google) { "plus" } else { "arrow.up.right" };
                    if !oslook::symbol(ui, sym, btn.center(), 11.0, bc) {
                        let bst = Stroke::new(1.5_f32, bc);
                        let b = btn.center();
                        ui.painter().line_segment([b - Vec2::new(4.5, 0.0), b + Vec2::new(4.5, 0.0)], bst);
                        ui.painter().line_segment([b - Vec2::new(0.0, 4.5), b + Vec2::new(0.0, 4.5)], bst);
                    }
                }
                let tip = match view {
                    View::Source(Source::Google) => Some(t(K::BrowseGoogle)),
                    View::Source(Source::Adobe) => Some(t(K::AdobeHint)),
                    _ => None,
                };
                let resp = match tip {
                    Some(tip) => resp.on_hover_text(tip),
                    None => resp,
                };
                if resp.clicked() {
                    let on_btn = service && resp.interact_pointer_pos().is_some_and(|p| btn.expand(3.0).contains(p));
                    if on_btn && view == View::Source(Source::Google) {
                        self.gf.open = true;
                        gfonts::start_list(&self.gf.shared);
                    } else if on_btn {
                        crate::paths::open_external("https://fonts.adobe.com");
                    } else {
                        self.view = view;
                        // An empty Google Fonts source opens the browser straight away.
                        if view == View::Source(Source::Google) && count == 0 {
                            self.gf.open = true;
                            gfonts::start_list(&self.gf.shared);
                        }
                    }
                }
            }
            ui.add_space(8.0);
        }
        self.sidebar_libraries(ui);
    }

    /// The top bar, left to right: search (with the system's magnifying glass) | "Frase" and the
    /// phrase's field | sentence, capitals, small letters and numbers | grid and list views |
    /// size, target app, light/dark, print and settings (as before, at the right edge).
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let lang = self.settings.lang;
        let t = move |k: K| i18n::tr(lang, k);
        let es = lang == 1;
        ui.horizontal_centered(|ui| {
            // 1. Search fonts: the field with the system's magnifying glass drawn inside it.
            let search = oslook::field(
                ui,
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text(t(K::SearchFonts))
                    .desired_width(180.0)
                    .margin(Margin { left: 26, right: 8, top: 5, bottom: 5 }),
            );
            let glass = Pos2::new(search.rect.left() + 14.0, search.rect.center().y);
            if !oslook::symbol(ui, "magnifyingglass", glass, 13.0, pal.muted) {
                paint_magnifier(ui.painter(), glass, pal.muted);
            }
            ui.separator();

            // 2. "Frase": brings the phrase back; its field follows (laid out below, from the right,
            //    so it takes the room the other groups leave).
            let phrase = self.settings.default_preview.clone();
            let on_phrase = self.preview == phrase;
            let frase = ui.selectable_label(on_phrase, presets(lang)[0].0);
            if frase.on_hover_text(if es { "Mostrar la frase" } else { "Show the phrase" }).clicked() {
                self.preview = phrase.clone();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // 6. The icons at the right edge, as they were: settings, print, light/dark, the app
                //    FontCraft works with, and the sample size.
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                if resp.hovered() || self.settings_open {
                    ui.painter().rect_filled(r, 6.0, pal.hover);
                }
                settings::paint_gear(ui.painter(), r.center(), pal.ink, if resp.hovered() || self.settings_open { pal.hover } else { pal.panel });
                if resp.on_hover_text(format!("{} (⌘,)", t(K::Settings))).clicked() {
                    self.settings_open = !self.settings_open;
                }
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                if resp.hovered() || self.print.is_some() {
                    ui.painter().rect_filled(r, 6.0, pal.hover);
                }
                settings::paint_printer(ui.painter(), r.center(), pal.ink, if resp.hovered() || self.print.is_some() { pal.hover } else { pal.panel });
                if resp.on_hover_text(format!("{} (⌘P)", t(K::Print))).clicked() {
                    self.open_print();
                }
                let dark = self.pal.dark;
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(r, 6.0, pal.hover);
                }
                settings::paint_theme_icon(ui.painter(), r.center(), dark, pal.ink, pal.panel);
                if resp.on_hover_text(if dark { t(K::SwitchLight) } else { t(K::SwitchDark) }).clicked() {
                    self.settings.theme = if dark { settings::ThemeChoice::Light } else { settings::ThemeChoice::Dark };
                    self.settings.save();
                }
                // The app FontCraft works with (Auto Activation): click to use the font there.
                self.target_button(ui);
                ui.separator();
                ui.label(egui::RichText::new(format!("{} px", self.size.round())).color(pal.ink).monospace());
                ui.label(egui::RichText::new("A").size(19.0).color(pal.muted));
                ui.spacing_mut().slider_width = 110.0;
                ui.add(egui::Slider::new(&mut self.size, mechanics::SIZE_MIN..=mechanics::SIZE_MAX).show_value(false));
                ui.label(egui::RichText::new("A").size(12.0).color(pal.muted));
                ui.separator();

                // 5. Grid and list views (right to left: list, then grid).
                let mode = self.settings.view_mode;
                let list_tip = if es { "Lista detallada" } else { "Detailed list" };
                if view_button(ui, &pal, mode == settings::ViewMode::List, list_tip, |p, c, col| {
                    for dy in [-5.0, 0.0, 5.0] {
                        p.rect_filled(Rect::from_center_size(c + Vec2::new(-6.0, dy), Vec2::splat(2.2)), 0.5, col);
                        p.line_segment([c + Vec2::new(-2.0, dy), c + Vec2::new(6.5, dy)], Stroke::new(1.4, col));
                    }
                }) {
                    self.settings.view_mode = settings::ViewMode::List;
                    self.settings.save();
                }
                // Samples (in the middle, as Font Book places its sample view): each family's sample
                // in a row, drawn as a large "A" beside lines of text.
                let samples_tip = if es { "Muestras" } else { "Samples" };
                if view_button(ui, &pal, mode == settings::ViewMode::Samples, samples_tip, |p, c, col| {
                    p.text(c + Vec2::new(-3.5, 0.5), Align2::CENTER_CENTER, "A", FontId::proportional(15.0), col);
                    for (dy, w) in [(-3.5, 5.0), (0.5, 5.0), (4.5, 3.5)] {
                        p.line_segment([c + Vec2::new(2.5, dy), c + Vec2::new(2.5 + w, dy)], Stroke::new(1.4, col));
                    }
                }) {
                    self.settings.view_mode = settings::ViewMode::Samples;
                    self.settings.save();
                }
                let grid_tip = if es { "Cuadrícula" } else { "Grid" };
                if view_button(ui, &pal, mode == settings::ViewMode::Grid, grid_tip, |p, c, col| {
                    for (dx, dy) in [(-4.0, -4.0), (4.0, -4.0), (-4.0, 4.0), (4.0, 4.0)] {
                        p.rect_filled(Rect::from_center_size(c + Vec2::new(dx, dy), Vec2::splat(5.0)), 1.0, col);
                    }
                }) {
                    self.settings.view_mode = settings::ViewMode::Grid;
                    self.settings.save();
                }
                ui.separator();

                // 4. Sentence, capitals, small letters and numbers (right to left).
                let tips: [&str; 5] = if es {
                    ["", "Oración", "Mayúsculas", "Minúsculas", "Números"]
                } else {
                    ["", "Sentence", "Capitals", "Small letters", "Numbers"]
                };
                for (i, (name, text)) in presets(lang).into_iter().enumerate().skip(1).rev() {
                    let on = self.preview == text;
                    if ui.selectable_label(on, name).on_hover_text(tips[i]).clicked() {
                        self.preview = text.to_string();
                    }
                }
                ui.separator();

                // 3. The phrase's field: shows the sample, and what the person types becomes the
                //    phrase "Frase" brings back.
                let w = (ui.available_width() - 6.0).max(80.0);
                let resp = oslook::field(
                    ui,
                    egui::TextEdit::singleline(&mut self.preview)
                        .hint_text(t(K::TypeToPreview))
                        .desired_width(w)
                        .margin(Margin::symmetric(8, 5)),
                );
                if resp.changed() && !presets(lang).iter().skip(1).any(|(_, p)| *p == self.preview) {
                    self.settings.default_preview = self.preview.clone();
                }
                if resp.lost_focus() {
                    self.settings.save();
                }
            });
        });
    }

    /// How tall a font's sample is, in ems (cached by face and text).
    fn extent(&mut self, face: usize, text: &str) -> f32 {
        if let Some(e) = self.extents.get(&(face, text.to_string())) {
            return *e;
        }
        let f = &self.lib.faces[face];
        let e = fonts::sample_extent(&self.lib.files[f.file].data, f.index, text);
        self.extents.insert((face, text.to_string()), e);
        e
    }

    fn list(&mut self, ui: &mut egui::Ui) {
        if self.settings.view_mode == settings::ViewMode::List {
            self.list_table(ui);
            return;
        }
        if self.settings.view_mode == settings::ViewMode::Grid {
            self.grid_view(ui);
            return;
        }
        let mut list_action: Option<menus::MenuAction> = None;
        let mut native_row: Option<Row> = None;
        let pal = self.pal;
        let lang = self.settings.lang;
        let t = move |k: K| i18n::tr(lang, k);
        let rows = self.rows();
        if rows.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new(t(K::NoMatch)).size(17.0).color(pal.muted));
            });
            return;
        }
        let pad = self.settings.row_padding();
        // Where the sample starts below the row's top, and the space left under it.
        // RightFont's row (0x100068570): sample from 23 pt below the top to 5 pt above the bottom;
        // the row-spacing setting adds or takes room around it.
        let img_off = mechanics::ROW_SAMPLE_TOP + (pad - 40.0) / 2.0;
        let bottom_gap = mechanics::ROW_SAMPLE_BOTTOM + (pad - 40.0) / 2.0;
        // Each row is as tall as its own sample needs at the chosen size (fonts with tall accents
        // or long descenders get more room), so no sample is cut off at the top or bottom.
        let heights: Vec<f32> = rows
            .iter()
            .map(|row| {
                let (family, face) = match *row {
                    Row::Family(f) => (f, self.lib.families[f].default_face),
                    Row::Face(f, face) => (f, face),
                };
                let text = self.preview_text(family);
                let ext = self.extent(face, &text);
                (img_off + (self.size * ext).ceil() + bottom_gap).max(img_off + 18.0 + bottom_gap)
            })
            .collect();
        let total: f32 = heights.iter().sum();
        let ctx = ui.ctx().clone();
        egui::ScrollArea::vertical().auto_shrink(false).show_viewport(ui, |ui, viewport| {
            ui.set_min_height(total);
            let origin = ui.max_rect().min;
            let w = ui.available_width();
            let mut y = 0.0;
            for (k, row) in rows.iter().enumerate() {
                let row_h = heights[k];
                let top = y;
                y += row_h;
                if top + row_h < viewport.min.y || top > viewport.max.y {
                    continue;
                }
                let rect = Rect::from_min_size(Pos2::new(origin.x, origin.y + top), Vec2::new(w, row_h));
                let resp = ui.interact(rect, ui.scope_id().with(("row", k)), Sense::click());
                let (family, face, indent) = match *row {
                    Row::Family(f) => (f, self.lib.families[f].default_face, 0.0),
                    Row::Face(f, face) => (f, face, 28.0),
                };
                let sel = self.selected == Some(*row);
                let p = ui.painter().clone();
                if sel {
                    p.rect_filled(rect, 0.0, pal.accent_soft);
                } else if resp.hovered() {
                    p.rect_filled(rect, 0.0, pal.hover);
                } else if matches!(row, Row::Face(..)) {
                    p.rect_filled(rect, 0.0, pal.face_bg);
                }
                p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0_f32, pal.line));

                let left = rect.left() + 20.0 + indent;
                let head_y = rect.top() + mechanics::ROW_ICON_Y + (pad - 40.0) / 4.0;
                let fam_name = self.lib.families[family].name.clone();
                let fam = &self.lib.families[family];
                let n_faces = fam.faces.len();
                let tri_rect = Rect::from_center_size(Pos2::new(rect.left() + 10.0, head_y), Vec2::splat(18.0));
                let star_rect = Rect::from_center_size(Pos2::new(rect.right() - mechanics::STAR_FROM_RIGHT, head_y), Vec2::splat(libs::ICON + 4.0));
                let is_fav = self.favorites.contains(&fam.name);
                // The icons sit side by side, all 18 points: asterisk or padlock, then the format.
                let format_c = Pos2::new(left + 31.0, head_y);
                let name_x = left + 48.0;
                match *row {
                    Row::Family(_) => {
                        if n_faces > 1 {
                            oslook::disclosure(ui, tri_rect.center(), self.expanded.contains(&family), pal.muted);
                        }
                        let name = p.text(Pos2::new(name_x, head_y), Align2::LEFT_CENTER, &fam.name, FontId::proportional(13.5), pal.ink);
                        let styles = if !self.settings.show_style_count {
                            String::new()
                        } else if n_faces == 1 {
                            t(K::Style1).to_string()
                        } else {
                            t(K::StylesN).replace("{n}", &n_faces.to_string())
                        };
                        p.text(Pos2::new(name.right() + 10.0, head_y), Align2::LEFT_CENTER, styles, FontId::proportional(12.5), pal.muted);
                        let star_hover = ui.rect_contains_pointer(star_rect);
                        let star_col = if is_fav {
                            libs::STAR_ON
                        } else if star_hover {
                            libs::STAR_ON.gamma_multiply(0.6)
                        } else {
                            libs::ASTERISK_GRAY
                        };
                        libs::paint_star(&p, star_rect.center(), is_fav, star_col);
                        // Fonts that have the character marked on the Glyphs page: a red dot left of the star.
                        if self.mark_char.is_some() && self.family_has_mark(family) {
                            let c = star_rect.center() - Vec2::new(libs::ICON + 8.0, 0.0);
                            App::paint_mark_dot(&p, c);
                            let mc = self.mark_char.map(|c| c.to_string()).unwrap_or_default();
                            ui.interact(Rect::from_center_size(c, Vec2::splat(libs::ICON)), ui.scope_id().with(("mark", k)), Sense::hover())
                                .on_hover_text(t(K::HasChar).replace("{c}", &mc));
                        }
                    }
                    Row::Face(_, face) => {
                        p.text(Pos2::new(name_x, head_y), Align2::LEFT_CENTER, &self.lib.faces[face].style, FontId::proportional(13.0), pal.muted);
                    }
                }
                // Asterisk (switch the font on/off) or padlock (can't be switched).
                self.activation_control(ui, *row, Pos2::new(left + 9.0, head_y));
                // The font's format.
                let fmt = {
                    let f = &self.lib.faces[face];
                    fonts::format_label(&self.lib.files[f.file].data, f.index)
                };
                libs::paint_format(&p, format_c, fmt, libs::ASTERISK_GRAY);
                ui.interact(Rect::from_center_size(format_c, Vec2::splat(libs::ICON)), ui.scope_id().with(("fmt", k)), Sense::hover())
                    .on_hover_text(format!("{}: {fmt}", t(K::Format)));

                let text = self.preview_text(family);
                let max_w = rect.right() - left - mechanics::ROW_SIDE;
                let (tex, size, drew, missing) = self.texture(&ctx, face, &text, self.size, max_w);
                if missing && self.settings.warn_missing {
                    p.text(Pos2::new(rect.right() - 44.0, head_y), Align2::RIGHT_CENTER, t(K::MissingLetters), FontId::proportional(12.0), pal.warn);
                }
                let img_top = rect.top() + img_off;
                if let Some(tex) = tex {
                    let clip = Rect::from_min_max(Pos2::new(left, rect.top() + 22.0), Pos2::new(rect.right() - mechanics::ROW_SIDE, rect.bottom()));
                    let img_rect = Rect::from_min_size(Pos2::new(left, img_top), size);
                    p.with_clip_rect(clip.intersect(p.clip_rect())).image(
                        tex.id(),
                        img_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        pal.ink,
                    );
                } else if !drew {
                    p.text(Pos2::new(left, img_top + 10.0), Align2::LEFT_TOP, t(K::CantShow), FontId::proportional(13.0), pal.muted);
                }

                if resp.clicked() {
                    let pos = resp.interact_pointer_pos().unwrap_or_default();
                    if matches!(row, Row::Family(_)) && star_rect.expand(2.0).contains(pos) {
                        let name = fam_name.clone();
                        if !self.favorites.remove(&name) {
                            self.favorites.insert(name);
                        }
                        self.save_favorites();
                    } else if matches!(row, Row::Family(_)) && n_faces > 1 && tri_rect.expand(4.0).contains(pos) {
                        self.toggle_expanded(family);
                    } else {
                        self.selected = Some(*row);
                        if let Some(app) = self.target_app() {
                            self.apply_to_app(app, face);
                        }
                    }
                }
                if resp.double_clicked() {
                    if matches!(row, Row::Family(_)) && n_faces > 1 {
                        self.toggle_expanded(family);
                    }
                }
                // Right-click: the font menu, as the system's own menu where it offers one.
                let row = *row;
                if nativemenu::available() {
                    if resp.secondary_clicked() {
                        self.selected = Some(row);
                        native_row = Some(row);
                    }
                } else {
                    resp.context_menu(|ui| {
                        self.selected = Some(row);
                        if let Some(a) = self.font_menu(ui, row) {
                            list_action = Some(a);
                        }
                    });
                }
            }
        });
        if let Some(row) = native_row {
            let (items, acts) = self.font_menu_native(row);
            self.open_native_menu(items, acts);
        }
        if let Some(a) = list_action {
            self.run_menu_action(a);
        }
    }

    fn family_date(&self, family: usize) -> String {
        let face_idx = self.lib.families[family].default_face;
        let face = &self.lib.faces[face_idx];
        let file = &self.lib.files[face.file];
        if let Ok(meta) = std::fs::metadata(&file.path) {
            if let Ok(time) = meta.created().or_else(|_| meta.modified()) {
                if let Ok(duration) = time.duration_since(std::time::UNIX_EPOCH) {
                    let (y, m, d) = days_to_ymd(duration.as_secs() / 86400);
                    return format!("{y:04}-{m:02}-{d:02}");
                }
            }
        }
        "-".to_string()
    }

    fn list_table(&mut self, ui: &mut egui::Ui) {
        let mut list_action: Option<menus::MenuAction> = None;
        let mut native_row: Option<Row> = None;
        let pal = self.pal;
        let lang = self.settings.lang;
        let t = move |k: K| i18n::tr(lang, k);
        let rows = self.rows();
        if rows.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new(t(K::NoMatch)).size(17.0).color(pal.muted));
            });
            return;
        }

        let total_w = ui.available_width();
        let col_icon = 54.0_f32;
        let col_name = 170.0_f32;
        let col_styles = 80.0_f32;
        let col_fmt = 65.0_f32;
        let col_class = 110.0_f32;
        let col_date = 100.0_f32;
        let fixed_w = col_icon + col_name + col_styles + col_fmt + col_class + col_date;
        let col_preview = (total_w - fixed_w).max(180.0);

        // Header
        let (hdr_rect, _) = ui.allocate_exact_size(Vec2::new(total_w, 28.0), Sense::hover());
        ui.painter().rect_filled(hdr_rect, 0.0, pal.status);
        ui.painter().line_segment([hdr_rect.left_bottom(), hdr_rect.right_bottom()], Stroke::new(1.0, pal.line));

        let x_icon = hdr_rect.left();
        let x_name = x_icon + col_icon;
        let x_preview = x_name + col_name;
        let x_styles = x_preview + col_preview;
        let x_fmt = x_styles + col_styles;
        let x_class = x_fmt + col_fmt;
        let x_date = x_class + col_class;

        let hdr_y = hdr_rect.center().y;
        let hdr_font = FontId::proportional(11.5);
        let hdr_col = pal.muted;
        ui.painter().text(Pos2::new(x_icon + col_icon / 2.0, hdr_y), Align2::CENTER_CENTER, if lang == 1 { "Estado" } else { "Status" }, hdr_font.clone(), hdr_col);
        ui.painter().text(Pos2::new(x_name + 4.0, hdr_y), Align2::LEFT_CENTER, if lang == 1 { "Nombre" } else { "Name" }, hdr_font.clone(), hdr_col);
        ui.painter().text(Pos2::new(x_preview + 4.0, hdr_y), Align2::LEFT_CENTER, if lang == 1 { "Vista previa" } else { "Preview" }, hdr_font.clone(), hdr_col);
        ui.painter().text(Pos2::new(x_styles + 4.0, hdr_y), Align2::LEFT_CENTER, if lang == 1 { "Estilos" } else { "Styles" }, hdr_font.clone(), hdr_col);
        ui.painter().text(Pos2::new(x_fmt + 4.0, hdr_y), Align2::LEFT_CENTER, if lang == 1 { "Formato" } else { "Format" }, hdr_font.clone(), hdr_col);
        ui.painter().text(Pos2::new(x_class + 4.0, hdr_y), Align2::LEFT_CENTER, if lang == 1 { "Clasificación" } else { "Classification" }, hdr_font.clone(), hdr_col);
        ui.painter().text(Pos2::new(x_date + 4.0, hdr_y), Align2::LEFT_CENTER, if lang == 1 { "Fecha de adición" } else { "Date Added" }, hdr_font, hdr_col);

        let row_h = 44.0_f32;
        let total_h = rows.len() as f32 * row_h;
        let ctx = ui.ctx().clone();

        egui::ScrollArea::vertical().auto_shrink(false).show_viewport(ui, |ui, viewport| {
            ui.set_min_height(total_h);
            let origin = ui.max_rect().min;
            let w = ui.available_width();

            for (k, row) in rows.iter().enumerate() {
                let top = k as f32 * row_h;
                if top + row_h < viewport.min.y || top > viewport.max.y {
                    continue;
                }
                let rect = Rect::from_min_size(Pos2::new(origin.x, origin.y + top), Vec2::new(w, row_h));
                let resp = ui.interact(rect, ui.scope_id().with(("list-row", k)), Sense::click());
                let (family, face, indent) = match *row {
                    Row::Family(f) => (f, self.lib.families[f].default_face, 0.0),
                    Row::Face(f, face) => (f, face, 20.0),
                };
                let sel = self.selected == Some(*row);
                let p = ui.painter().clone();
                if sel {
                    p.rect_filled(rect, 0.0, pal.accent_soft);
                } else if resp.hovered() {
                    p.rect_filled(rect, 0.0, pal.hover);
                } else if matches!(row, Row::Face(..)) {
                    p.rect_filled(rect, 0.0, pal.face_bg);
                }
                p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, pal.line));

                let row_cy = rect.center().y;
                let (fam_name, n_faces, fam_class, fam_mono) = {
                    let fam = &self.lib.families[family];
                    (fam.name.clone(), fam.faces.len(), fam.class, fam.monospaced)
                };

                // 1. Icon (disclosure chevron for multi-face families, padlock or asterisk)
                let tri_c = Pos2::new(rect.left() + 8.0, row_cy);
                let act_c = Pos2::new(rect.left() + 26.0, row_cy);
                if matches!(row, Row::Family(_)) && n_faces > 1 {
                    oslook::disclosure(ui, tri_c, self.expanded.contains(&family), pal.muted);
                    let tri_rect = Rect::from_center_size(tri_c, Vec2::splat(16.0));
                    if resp.clicked() && resp.interact_pointer_pos().is_some_and(|p| tri_rect.expand(2.0).contains(p)) {
                        self.toggle_expanded(family);
                    }
                }
                self.activation_control(ui, *row, act_c);

                // 2. Name
                let name_x = x_name + 4.0 + indent;
                match *row {
                    Row::Family(_) => {
                        p.text(Pos2::new(name_x, row_cy), Align2::LEFT_CENTER, &fam_name, FontId::proportional(13.5), pal.ink);
                    }
                    Row::Face(_, face) => {
                        p.text(Pos2::new(name_x, row_cy), Align2::LEFT_CENTER, &self.lib.faces[face].style, FontId::proportional(13.0), pal.muted);
                    }
                }

                // 3. Specimen Preview
                let text = self.preview_text(family);
                let preview_sz = (self.size * 0.7).clamp(16.0, 26.0);
                let p_max_w = (col_preview - 12.0).max(10.0);
                let (tex, size, _drew, _missing) = self.texture(&ctx, face, &text, preview_sz, p_max_w);
                if let Some(tex) = tex {
                    let scale = (p_max_w / size.x).min(32.0 / size.y).min(1.0);
                    let draw_size = size * scale;
                    let preview_rect = Rect::from_min_size(
                        Pos2::new(x_preview + 6.0, row_cy - draw_size.y / 2.0),
                        draw_size,
                    );
                    p.image(
                        tex.id(),
                        preview_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        pal.ink,
                    );
                }

                // 4. Styles count
                if matches!(row, Row::Family(_)) {
                    let st_str = if n_faces == 1 {
                        if lang == 1 { "1 estilo".to_string() } else { "1 style".to_string() }
                    } else {
                        if lang == 1 { format!("{n_faces} estilos") } else { format!("{n_faces} styles") }
                    };
                    p.text(Pos2::new(x_styles + 4.0, row_cy), Align2::LEFT_CENTER, st_str, FontId::proportional(12.0), pal.muted);
                } else {
                    p.text(Pos2::new(x_styles + 4.0, row_cy), Align2::LEFT_CENTER, "-", FontId::proportional(12.0), pal.muted);
                }

                // 5. Format badge / label
                let fmt = {
                    let f_obj = &self.lib.faces[face];
                    let file = &self.lib.files[f_obj.file];
                    fonts::format_label(&file.data, f_obj.index)
                };
                p.text(Pos2::new(x_fmt + 4.0, row_cy), Align2::LEFT_CENTER, fmt, FontId::proportional(11.5), pal.muted);

                // 6. Classification
                let class_str = class_label(lang, fam_class, fam_mono);
                p.text(Pos2::new(x_class + 4.0, row_cy), Align2::LEFT_CENTER, class_str, FontId::proportional(12.0), pal.muted);

                // 7. Date added
                let date_str = self.family_date(family);
                p.text(Pos2::new(x_date + 4.0, row_cy), Align2::LEFT_CENTER, date_str, FontId::proportional(11.5), pal.muted);

                if resp.clicked() {
                    self.selected = Some(*row);
                }
                if resp.double_clicked() {
                    if let Row::Family(f) = *row {
                        if self.lib.families[f].faces.len() > 1 {
                            self.toggle_expanded(f);
                        }
                    }
                }
                let row = *row;
                if nativemenu::available() {
                    if resp.secondary_clicked() {
                        self.selected = Some(row);
                        native_row = Some(row);
                    }
                } else {
                    resp.context_menu(|ui| {
                        self.selected = Some(row);
                        if let Some(a) = self.font_menu(ui, row) {
                            list_action = Some(a);
                        }
                    });
                }
            }
        });

        if let Some(row) = native_row {
            let (items, acts) = self.font_menu_native(row);
            self.open_native_menu(items, acts);
        }
        if let Some(a) = list_action {
            self.run_menu_action(a);
        }
    }
}

impl eframe::App for App {
    /// Test pictures: FONTCRAFT_TEST_HOVER="x,y" keeps the pointer there (read once).
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        static HOVER: std::sync::OnceLock<Option<egui::Pos2>> = std::sync::OnceLock::new();
        let at = HOVER.get_or_init(|| {
            let v: Vec<f32> = std::env::var("FONTCRAFT_TEST_HOVER").ok()?.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            match v[..] {
                [x, y] => Some(egui::pos2(x, y)),
                _ => None,
            }
        });
        if let Some(p) = at {
            raw.events.push(egui::Event::PointerMoved(*p));
        }
    }

    /// When FontCraft quits: the "turn off when FontCraft quits" options.
    #[cfg(not(target_os = "android"))]
    fn on_exit(&mut self) {
        self.autoact_quit();
    }

    /// Runs before every pass, and alone while the window is put away in the menu bar or covered
    /// by other windows (eframe 0.37 runs no ui then): FontCraft keeps working in the background —
    /// the menu bar asterisk, turning fonts off when their app or document closes, and the Missing
    /// Fonts window, which is drawn even then (RequestPaintWhileHidden).
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.screenshot.is_some() {
            return;
        }
        // Menu bar icon and "open FontCraft" shortcut.
        let keep = self.settings.background || self.start_hidden;
        self.bg.sync(ctx, keep, &self.settings.hotkey);
        // Background start: put away the window eframe shows after its first frame.
        if self.hide_at_start {
            if cfg!(target_os = "macos") {
                if background::main_window_up() {
                    self.hide_at_start = false;
                    background::hide_app();
                } else {
                    ctx.request_repaint_after(std::time::Duration::from_millis(50));
                }
            } else if self.frame >= 1 {
                self.hide_at_start = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            } else {
                ctx.request_repaint();
            }
        }
        // Windows and Linux: clicking the menu bar icon (or the shortcut) opens the window full
        // screen; clicking it again while the window is in front puts it away.
        if self.bg.wake.swap(false, std::sync::atomic::Ordering::SeqCst) {
            background::set_in_background(false);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }
        if self.bg.sleep.swap(false, std::sync::atomic::Ordering::SeqCst) {
            self.go_to_menu_bar(ctx);
        }
        // With "keep running in the menu bar", closing the window only puts it away.
        if keep && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.go_to_menu_bar(ctx);
        }
        // The menu bar asterisk follows whether the window is in front (checked twice a second).
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        if ctx.input(|i| i.viewport().visible()).unwrap_or(true) {
            // The window is up: ui() does the rest itself.
            return;
        }
        self.autoact_start();
        self.autoact_tick(ctx);
        self.missing_tick();
        if self.missing.open && self.missing.bring_front > 0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
            ctx.request_repaint();
        }
    }

    /// Android builds eframe with its OpenGL (glow) renderer, whose `on_exit` gets the GL context.
    #[cfg(target_os = "android")]
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.autoact_quit();
    }

    /// eframe 0.37 hands the app its window's root `Ui`; FontCraft lays out its panels in it.
    fn ui(&mut self, root: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(h) = frame.window_handle() {
                match h.as_raw() {
                    RawWindowHandle::AppKit(a) => {
                        self.ns_view = a.ns_view.as_ptr();
                        nativemenu::set_window(self.ns_view as usize);
                        background::set_main_view(self.ns_view);
                    }
                    RawWindowHandle::Win32(w) => nativemenu::set_window(w.hwnd.get() as usize),
                    _ => {}
                }
            }
        }
        self.frame += 1;

        if let Some(path) = self.screenshot.clone() {
            // Keeps drawing while unfocused (and while macOS draws a hidden window slowly).
            ctx.request_repaint();
            let now = ctx.input(|i| i.time);
            let started = *self.shot_started.get_or_insert(now);
            // Waits a little longer while the Google Fonts list is still arriving.
            let waiting = ((self.gf.open && self.gf.shared.lock().unwrap().loading) || self.print.as_ref().is_some_and(|p| p.loading())) && self.frame < 900 && now - started < 20.0;
            // About 120 frames, or 4 seconds when the window draws slowly (covered, or not in
            // front): a covered window on macOS gets few frames, and waiting for 120 of them took
            // longer than the check allows. A picture asked for and not delivered within 3 s is
            // asked for again; after 60 s FontCraft gives up and quits.
            let ready = (self.frame >= 120 || (now - started >= 4.0 && self.frame >= 10)) && !waiting;
            if self.shot_requested && now - self.shot_asked_at > 3.0 {
                self.shot_requested = false;
            }
            if now - started > 60.0 {
                eprintln!("screenshot: no picture after 60 s");
                std::process::exit(1);
            }
            if ready && !self.shot_requested {
                self.shot_requested = true;
                self.shot_asked_at = now;
                // eframe 0.37 delivers the picture to a callback; it is kept until the next frame.
                let slot = self.shot_image.clone();
                let repaint = ctx.clone();
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |img| {
                    *slot.lock().unwrap() = Some(img);
                    repaint.request_repaint();
                })));
            }
            let shot = self.shot_image.lock().unwrap().take();
            if let Some(img) = shot {
                let [w, h] = img.size;
                let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                if let Some(buf) = image::RgbaImage::from_raw(w as u32, h as u32, bytes) {
                    if let Some(dir) = path.parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    if let Err(e) = buf.save(&path) {
                        eprintln!("could not save screenshot: {e}");
                    }
                }
                std::process::exit(0);
            }
        }

        self.apply_theme(ctx);
        let pal = self.pal;
        let lang = self.settings.lang;
        let t = move |k: K| i18n::tr(lang, k);
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma)) {
            self.settings_open = !self.settings_open;
        }
        // ⌘+ / ⌘− change the preview size by RightFont's step, within the slider's limits.
        let bigger = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Equals) || i.consume_key(egui::Modifiers::COMMAND, egui::Key::Plus));
        let smaller = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Minus));
        if bigger {
            self.size = (self.size + mechanics::SIZE_STEP).min(mechanics::SIZE_MAX);
        }
        if smaller {
            self.size = (self.size - mechanics::SIZE_STEP).max(mechanics::SIZE_MIN);
        }

        self.google_sync(ctx);

        // The menu bar icon and the "open FontCraft" shortcut are looked after in `logic`, which
        // also runs while the window is put away.
        self.autoact_start();
        self.autoact_tick(ctx);
        // First start: when FontCraft isn't allowed to work with other apps yet, macOS shows its
        // own request (nothing opens when it already is). Apple Events permission is asked by
        // macOS itself the first time FontCraft talks to each app.
        if self.screenshot.is_none() && self.frame == 30 && !self.settings.asked_access {
            self.settings.asked_access = true;
            self.settings.save();
            #[cfg(target_os = "macos")]
            if !autoact::trusted() {
                let _ = autoact::trusted_or_ask();
            }
        }
        // A macOS newer than the ones FontCraft was tested on: said once per system version, as
        // RightFont warns (+[RFGlobals isCompatibleSystemVersion], 0x1000e97f0).
        if self.screenshot.is_none() && self.frame == 40 {
            if let Some(v) = mechanics::macos_version().filter(|v| !mechanics::is_compatible_macos(v)) {
                let seen = paths::data_dir().join("untested-system.txt");
                if std::fs::read_to_string(&seen).map(|s| s.trim() != v).unwrap_or(true) {
                    self.notice = Some(i18n::tr(self.settings.lang, K::UntestedSystem).replace("{v}", &v));
                    let _ = std::fs::write(&seen, &v);
                }
            }
        }
        self.watch_libraries(ctx);
        if let Some(text) = self.clip.take() {
            ctx.copy_text(text);
        }
        if !(self.settings_open && self.settings_tab == SettingsTab::AutoActivation) {
            self.autoact.settings_seen = false;
        }

        egui::Panel::top("toolbar")
            .exact_size(50.0)
            .frame(egui::Frame::NONE.fill(pal.panel).inner_margin(Margin::symmetric(14, 0)).stroke(Stroke::new(1.0_f32, pal.line)))
            .show(root, |ui| self.toolbar(ui));

        let mut left_open = self.left_open;
        let left = egui::Panel::left("sidebar")
            .exact_size(self.settings.left_w)
            .resizable(false)
            // Finder's sidebar on macOS 26: a card floating over the window, inset from its edges.
            .frame(egui::Frame::NONE.fill(pal.panel).inner_margin(Margin::same(finder::CARD_INSET)))
            .show_collapsible(root, &mut left_open, |ui| {
                let card = ui.max_rect();
                finder::paint_card(ui, card, &pal);
                let inner = card.shrink2(Vec2::new(finder::CARD_PAD, 0.0));
                ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
                    egui::ScrollArea::vertical().id_salt("sidebar-scroll").auto_shrink(false).show(ui, |ui| self.sidebar(ui));
                });
            });
        let left_edge = left.as_ref().map(|r| r.response.rect.right()).unwrap_or(ctx.content_rect().left());

        let shown = self.count(self.view);
        let styles: usize = (0..self.lib.families.len())
            .filter(|&i| self.family_matches(i, self.view))
            .map(|i| self.lib.families[i].faces.len())
            .sum();
        egui::Panel::bottom("status")
            .exact_size(26.0)
            .frame(egui::Frame::NONE.fill(pal.status).inner_margin(Margin::symmetric(14, 0)).stroke(Stroke::new(1.0_f32, pal.line)))
            .show(root, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(egui::RichText::new(t(K::StatusLine).replace("{n}", &shown.to_string()).replace("{m}", &styles.to_string())).size(12.5).color(pal.muted));
                    if let Some(n) = &self.notice {
                        ui.label(egui::RichText::new(n).size(12.5).color(pal.warn));
                    }
                });
            });

        let mut right_edge = None;
        if self.settings.show_details {
            // Never narrower than four columns of glyphs (it can be made wider, or folded away).
            let min_w = self.details_min_w();
            if self.settings.right_w < min_w {
                self.settings.right_w = min_w;
            }
            let mut right_open = self.right_open;
            let right = egui::Panel::right("details")
                .exact_size(self.settings.right_w)
                .resizable(false)
                // The same floating card and colour as the sidebar (Finder's look on macOS 26).
                .frame(egui::Frame::NONE.fill(pal.panel).inner_margin(Margin::same(finder::CARD_INSET)))
                .show_collapsible(root, &mut right_open, |ui| {
                    let card = ui.max_rect();
                    self.details_card = Some(card);
                    finder::paint_card(ui, card, &pal);
                    let inner = card.shrink2(Vec2::new(DETAILS_PAD, 2.0));
                    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| self.details(ui));
                });
            right_edge = Some(right.map(|r| r.response.rect.left()).unwrap_or(ctx.content_rect().right()));
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(pal.panel))
            .show(root, |ui| self.list(ui));

        // The tabs on the panel edges that fold a panel away or bring it back
        // (hidden while a window such as Settings or Google Fonts is open on top).
        // Holding a tab and dragging, or dragging a panel's edge, resizes the panel.
        let window_open = self.settings_open
            || self.new_lib.is_some()
            || self.gf.open
            || self.import_dlg.is_some()
            || self.char_info.is_some()
            || self.similar.is_some()
            || self.print.is_some()
            || self.confirm.is_some()
            || self.missing.open;
        let resize = |this: &mut App, left: bool, delta: Option<f32>, ended: bool| {
            if let Some(dx) = delta {
                let start = if left { this.settings.left_w } else { this.settings.right_w };
                let base = this.resizing.get_or_insert((left, start)).1;
                if left {
                    this.settings.left_w = (base + dx).clamp(150.0, 480.0);
                } else {
                    this.settings.right_w = (base - dx).clamp(this.details_min_w(), DETAILS_MAX_W);
                }
            }
            if ended {
                this.resizing = None;
                this.settings.save();
            }
        };
        if !window_open {
            if self.left_open {
                let (d, end) = panel_edge(ctx, "left-edge", left_edge);
                resize(self, true, d, end);
            }
            match panel_tab(ctx, "left-tab", left_edge, true, self.left_open, &pal, t(K::HidePanel), t(K::ShowPanel)) {
                TabAction::Toggle => self.left_open = !self.left_open,
                TabAction::Drag(dx) => resize(self, true, Some(dx), false),
                TabAction::DragEnd => resize(self, true, None, true),
                TabAction::None => {}
            }
        }
        if let Some(edge) = right_edge.filter(|_| !window_open) {
            if self.right_open {
                let (d, end) = panel_edge(ctx, "right-edge", edge);
                resize(self, false, d, end);
            }
            match panel_tab(ctx, "right-tab", edge, false, self.right_open, &pal, t(K::HidePanel), t(K::ShowPanel)) {
                TabAction::Toggle => self.right_open = !self.right_open,
                TabAction::Drag(dx) => resize(self, false, Some(dx), false),
                TabAction::DragEnd => resize(self, false, None, true),
                TabAction::None => {}
            }
        }

        // A glyph being dragged from the Glyphs page follows the pointer; dropped on the target
        // app's icon in the toolbar, it's inserted into that app's document.
        // Dragged past the window's edge: hand the glyph to the system's drag, so it can be dropped
        // into any app (as the character, in its font).
        if let Some((gid, face)) = self.autoact.dragging {
            let inside = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| ctx.content_rect().shrink(4.0).contains(p));
            if !inside && !self.drag_out_tried {
                self.drag_out_tried = true;
                if let Some(ch) = self.glyph_char(gid) {
                    let at = ctx.input(|i| i.pointer.latest_pos()).unwrap_or_default();
                    let font = self.face_names(face, true);
                    let rtf = autoact::rtf_for(&ch.to_string(), &font);
                    let png = {
                        let f = &self.lib.faces[face];
                        fonts::render_glyph(&self.lib.files[f.file].data, f.index, gid, 48.0 * ctx.pixels_per_point()).and_then(|r| {
                            let mut img = image::RgbaImage::new(r.w as u32, r.h as u32);
                            for (i, a) in r.alpha.iter().enumerate() {
                                img.put_pixel((i % r.w) as u32, (i / r.w) as u32, image::Rgba([0, 0, 0, *a]));
                            }
                            let mut out = std::io::Cursor::new(Vec::new());
                            img.write_to(&mut out, image::ImageFormat::Png).ok().map(|_| out.into_inner())
                        })
                    };
                    // Like RightFont (0x1000b68c8 writes the dragged glyph into the Caches folder), the
                    // drag also carries a picture file of the glyph, for apps that take images.
                    let file = png.as_ref().and_then(|bytes| {
                        let dir = paths::cache_dir();
                        std::fs::create_dir_all(&dir).ok()?;
                        let f = dir.join(format!("{} U+{:04X}.png", self.lib.faces[face].family.replace('/', "-"), ch as u32));
                        std::fs::write(&f, bytes).ok()?;
                        Some(f)
                    });
                    if dragout::start(self.ns_view, &ch.to_string(), Some(&rtf), png, file.as_deref(), (at.x as f64, at.y as f64)) {
                        // The font is turned on so the app it lands in can show it.
                        let files = self.files_for(face);
                        let _ = self.auto_on(&files);
                        self.autoact.dragging = None;
                    }
                }
            }
        } else {
            self.drag_out_tried = false;
        }
        if let Some((gid, face)) = self.autoact.dragging {
            if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                let (tex, size, _, _) = self.texture_glyph(ctx, face, gid, 48.0);
                if let Some(tex) = tex {
                    egui::Area::new(egui::Id::unique("glyph-drag")).order(egui::Order::Tooltip).fixed_pos(pos - size / 2.0).interactable(false).show(ctx, |ui| {
                        ui.add(egui::Image::new((tex.id(), size)).tint(pal.accent));
                    });
                }
            }
            if ctx.input(|i| i.pointer.any_released()) {
                let over = ctx.input(|i| i.pointer.hover_pos()).zip(self.autoact.target_rect).is_some_and(|(p, r)| r.contains(p));
                if over {
                    if let Some(app) = self.target_app() {
                        self.insert_glyph(app, face, gid);
                    }
                }
                self.autoact.dragging = None;
            }
        }

        self.settings_window(ctx);
        self.char_info_window(ctx);
        self.confirm_window(ctx);
        self.missing_tick();
        self.native_menu_wait(ctx);
        // FONTCRAFT_MENU_BENCH (test runs): how long the font menu takes to put together.
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if self.screenshot.is_some() && self.frame == 60 && std::env::var("FONTCRAFT_MENU_BENCH").is_ok() {
            if let Some(row) = self.rows().first().copied() {
                let t0 = std::time::Instant::now();
                let (items, _) = self.font_menu_native(row);
                let t1 = t0.elapsed();
                crate::nativemenu::build_only("bench", items);
                eprintln!("menu bench: items {:?}, system menu {:?}", t1, t0.elapsed() - t1);
            }
        }
        self.missing_window(ctx);
        self.print_window(ctx);
        self.list_pick_window(ctx);
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::P)) && self.print.is_none() {
            self.open_print();
        }
        self.similar_window(ctx);
        self.google_window(ctx);
        self.import_window(ctx);
        // A folder dropped onto the window starts a new library from it.
        if self.new_lib.is_none() && self.import_dlg.is_none() {
            if let Some(p) = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).find(|p| !p.as_os_str().is_empty())) {
                if p.is_dir() {
                    self.new_lib = Some(libs::NewLib::new(p.display().to_string()));
                }
            }
        }
        self.new_library_window(ctx);

        // Forget previews that haven't been on screen for a while.
        if self.cache.len() > 300 {
            let now = self.frame;
            self.cache.retain(|_, c| now - c.last < 60);
        }
        if self.cache.len() > 3000 {
            self.cache.clear();
        }
    }
}

#[cfg_attr(target_os = "android", allow(dead_code))]
fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    // `FontCraft --version`: its name and version, then quit (the installers' checks run it).
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("FontCraft {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    // Developer aid: turn the login item on or off, then quit.
    if let Some(i) = args.iter().position(|a| a == "--test-login-item") {
        background::set_login_item(args.get(i + 1).map(|v| v == "on").unwrap_or(false));
        return Ok(());
    }
    // Developer aid: FontCraft's PhotoCraft channel against a headless PhotoCraft, then quit.
    if let Some(i) = args.iter().position(|a| a == "--test-photocraft-channel") {
        if let (Some(port), Some(token)) = (args.get(i + 1), args.get(i + 2)) {
            autoact::photocraft::selftest(port, token);
        }
        return Ok(());
    }
    // Developer aid: PressCraft's font field through Accessibility (a running PressCraft), then quit.
    if let Some(i) = args.iter().position(|a| a == "--test-presscraft-field") {
        presscraft::selftest(args.get(i + 1).map(String::as_str));
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--test-presscraft-apply") {
        if let Some(family) = args.get(i + 1) {
            presscraft::selftest_apply(family, args.get(i + 2).map(String::as_str));
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--test-clipboard") {
        autoact::clipboard_selftest();
        return Ok(());
    }
    // Developer aid: download one Google Fonts family into the Google Fonts folder, then quit.
    if let Some(i) = args.iter().position(|a| a == "--google-download") {
        if let Some(name) = args.get(i + 1) {
            println!("{name}: {:?}", gfonts::download_now(name));
        }
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--test-activate") {
        if let Some(p) = args.get(i + 1) {
            let path = std::path::Path::new(p);
            println!("activate: {}", activate::set(path, true));
            println!("deactivate: {}", activate::set(path, false));
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--list-classes") {
        let lib = fonts::scan(&[]);
        for c in [
            fonts::Class::Sans,
            fonts::Class::Serif,
            fonts::Class::Slab,
            fonts::Class::Script,
            fonts::Class::Blackletter,
            fonts::Class::Display,
            fonts::Class::Symbol,
            fonts::Class::Other,
        ] {
            let names: Vec<&str> = lib.families.iter().filter(|f| f.class == c).map(|f| f.name.as_str()).collect();
            println!("{c:?} ({}): {}\n", names.len(), names.join(", "));
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--list-features") {
        let lib = fonts::scan(&[]);
        for fam in &lib.families {
            let f = &lib.faces[fam.default_face];
            if let Ok(tf) = ttf_parser::Face::parse(&lib.files[f.file].data, f.index) {
                let have: Vec<String> = ot::MAIN_FEATURES
                    .iter()
                    .filter(|t| ot::feature_detail(&tf, t).present)
                    .map(|t| String::from_utf8_lossy(*t).to_string())
                    .collect();
                let sets = ot::stylistic_sets(&tf);
                if !have.is_empty() || !sets.is_empty() {
                    println!("{} | {} | sets {:?}", fam.name, have.join(","), sets);
                }
            }
        }
        return Ok(());
    }
    // Developer aid: write every icon size (app and .fcl document, light and dark), then quit.
    if let Some(i) = args.iter().position(|a| a == "--make-icons") {
        let dir = PathBuf::from(args.get(i + 1).cloned().unwrap_or_else(|| "assets/icons".into()));
        match appicon::make_all(&dir) {
            Ok(made) => println!("icons: {}", made.join(", ")),
            Err(e) => eprintln!("icons: {e}"),
        }
        return Ok(());
    }
    // Developer aid: Android launcher icons, then quit.
    if let Some(i) = args.iter().position(|a| a == "--make-android-icons") {
        let dir = PathBuf::from(args.get(i + 1).cloned().unwrap_or_else(|| "android/res".into()));
        match appicon::make_android(&dir) {
            Ok(()) => println!("android icons: {}", dir.display()),
            Err(e) => eprintln!("android icons: {e}"),
        }
        return Ok(());
    }
    // Developer aid: lay out a catalog PDF of the first families, then quit.
    if let Some(i) = args.iter().position(|a| a == "--print-test") {
        let path = PathBuf::from(args.get(i + 1).cloned().unwrap_or_else(|| "catalog.pdf".into()));
        let lib = fonts::scan(&[]);
        match print::test_pdf(&lib, &path) {
            Ok(n) => println!("pdf: {n} pages"),
            Err(e) => eprintln!("pdf: {e}"),
        }
        return Ok(());
    }
    let screenshot = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from);
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([900.0, 560.0])
        .with_title("FontCraft")
        .with_icon(std::sync::Arc::new(appicon::window_icon(appicon::system_is_dark())));
    // --screenshot (the automatic checks): the window opens without taking focus, so a check never
    // gets in the way of what the person is doing; on Windows also behind the other windows (a
    // covered window still draws there; on macOS and Linux it may stop drawing, so it isn't lowered).
    if screenshot.is_some() {
        viewport = viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }
    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions { viewport, ..Default::default() };
    // The graphics device asks only for what FontCraft draws with (2D textures up to what the GPU
    // offers, at most 8192): small GPUs (Raspberry Pi, other ARM boards) refused the default
    // request ("max_texture_dimension_3d requested 2048, allowed 512").
    #[cfg(not(target_os = "android"))]
    {
        use eframe::egui_wgpu::{wgpu, WgpuSetup, WgpuSetupCreateNew};
        options.wgpu_options.wgpu_setup = WgpuSetup::CreateNew(WgpuSetupCreateNew {
            device_descriptor: std::sync::Arc::new(|adapter: &wgpu::Adapter| {
                let have = adapter.limits();
                let base = if adapter.get_info().backend == wgpu::Backend::Gl { wgpu::Limits::downlevel_webgl2_defaults() } else { wgpu::Limits::downlevel_defaults() };
                let mut limits = base.using_resolution(have.clone()).using_alignment(have.clone());
                limits.max_texture_dimension_2d = have.max_texture_dimension_2d.min(8192);
                wgpu::DeviceDescriptor { label: Some("FontCraft"), required_limits: limits, ..Default::default() }
            }),
            ..WgpuSetupCreateNew::without_display_handle()
        });
    }
    // macOS: the app doesn't come to the front either. Started in the background (the menu bar
    // asterisk kept): only in the menu bar, out of the Dock, until the asterisk is clicked, as
    // RightFont at login (Accessory, 0x1000bee24).
    #[cfg(target_os = "macos")]
    {
        let hidden = starts_in_background(&Settings::load(), screenshot.is_some());
        if screenshot.is_some() || hidden {
            options.event_loop_builder = Some(Box::new(move |b| {
                use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
                b.with_activate_ignoring_other_apps(false);
                if hidden {
                    b.with_activation_policy(ActivationPolicy::Accessory);
                }
            }));
        }
    }
    eframe::run_native("FontCraft", options, Box::new(move |cc| Ok(Box::new(App::new(cc, screenshot)))))
}

/// FontCraft starts in the background, only its menu bar asterisk showing, when "Keep FontCraft in
/// the menu bar" is on or it was started at login (--background); never for test pictures, and not
/// on Android or iPadOS (no menu bar there).
fn starts_in_background(settings: &Settings, screenshot: bool) -> bool {
    !screenshot
        && !cfg!(any(target_os = "android", target_os = "ios"))
        && (settings.background || std::env::args().any(|a| a == "--background"))
}

/// Android: the entry point the system calls (built as a library by `android/` and
/// `packaging/android/build-apk.sh`). FontCraft's data goes in the app's private folder; the fonts shown are the
/// system's (/system/fonts), libraries, folders and Google Fonts.
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    if let Some(dir) = app.internal_data_path() {
        std::env::set_var("HOME", dir);
    }
    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native("FontCraft", options, Box::new(|cc| Ok(Box::new(App::new(cc, None))))) {
        eprintln!("FontCraft Android error: {e}");
    }
}
