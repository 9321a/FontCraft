#![cfg_attr(not(target_os = "macos"), allow(dead_code))]
//! PressCraft (app.presscraft), the person's layout, illustration and photo app (Rust, eframe/egui
//! 0.37 with AccessKit). What FontCraft does with it comes from PressCraft's own code (the person's
//! project, read only from here; details in MECHANICS.md, "PressCraft"):
//!
//! - It reads the font folders once, when it opens (text.rs `scan_system`, a thread started in
//!   `App::new`; no font-change listener and no command to read them again): /System/Library/Fonts,
//!   /Library/Fonts and ~/Library/Fonts, four folders deep, so the fonts FontCraft keeps on in
//!   ~/Library/Fonts/FontCraft are in its list when they were there before it opened.
//! - The selected text's font: its text options have a "Fuente" field (studio2.rs `font_picker`, an
//!   egui TextEdit with that placeholder). What is typed there keeps the families starting with it
//!   first (`find_fonts`), Return chooses the first one, and `format_text` puts it on the selected
//!   text, the caret's paragraph or the selected text frames. PressCraft has no script and no control
//!   channel, and it pastes plain text only, so FontCraft fills that field through Accessibility
//!   (AccessKit gives egui's fields to macOS: AXPlaceholderValue, and AXFocused and AXValue can be
//!   set) and presses Return; the style comes from the style menu beside the field.
//! - Its documents are PDFs that carry the editable document (a zip holding document.json, fileio.rs
//!   `pdf_document_bytes`); older ones are that zip alone (.presscraft). Missing Fonts reads the
//!   families there. The window title names the front document ("Cartel — PressCraft", app.rs) and
//!   PressCraft's recent list (settings.json "recent") has its full path.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const BUNDLE: &str = "app.presscraft";
/// The text options' font field, found by its placeholder (studio2.rs `font_picker`:
/// `hint_text("Fuente")`).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const FONT_FIELD: &str = "Fuente";

/// What PressCraft did with a font FontCraft set in its field.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub enum Answer {
    /// The text took the family (and the style, when PressCraft has it).
    Done,
    /// PressCraft doesn't list that family (it reads the fonts when it opens): nothing changed.
    NotInList,
    /// No font field showing: no text tool, and no text selected or being edited.
    NoField,
    /// Accessibility can't reach PressCraft (closed, or FontCraft not allowed).
    Unreachable,
}

/// The editable document inside a PressCraft PDF: the zip whose first file is "mimetype" holding
/// application/x-presscraft, up to the end of its central directory (the way PressCraft itself
/// finds it when another program rewrote the PDF, fileio.rs `pdf_document_bytes`). An older
/// .presscraft file is that zip from its first byte.
pub fn document_zip(bytes: &[u8]) -> Option<&[u8]> {
    let find = |hay: &[u8], needle: &[u8], from: usize| -> Option<usize> {
        (from < hay.len()).then(|| hay[from..].windows(needle.len()).position(|w| w == needle).map(|i| i + from)).flatten()
    };
    let mut from = 0;
    while let Some(z) = find(bytes, b"PK\x03\x04", from) {
        let head = &bytes[z..(z + 120).min(bytes.len())];
        if bytes.get(z + 30..z + 38) == Some(b"mimetype".as_slice()) && find(head, b"application/x-presscraft", 0).is_some() {
            let eocd = find(bytes, b"PK\x05\x06", z)?;
            let comment = bytes.get(eocd + 20..eocd + 22).map(|c| u16::from_le_bytes([c[0], c[1]]) as usize).unwrap_or(0);
            return bytes.get(z..eocd + 22 + comment);
        }
        from = z + 4;
    }
    None
}

/// Every font family a PressCraft document names (its text formats, character styles and
/// tables: the "family" fields of document.json), sorted.
pub fn families(bytes: &[u8]) -> Vec<String> {
    use std::io::Read;
    let mut out = BTreeSet::new();
    let Some(zip) = document_zip(bytes) else { return Vec::new() };
    let Ok(mut z) = zip::ZipArchive::new(std::io::Cursor::new(zip)) else { return Vec::new() };
    let Ok(mut f) = z.by_name("document.json") else { return Vec::new() };
    let mut json = Vec::new();
    if f.read_to_end(&mut json).is_err() {
        return Vec::new();
    }
    let Ok(doc) = serde_json::from_slice::<serde_json::Value>(&json) else { return Vec::new() };
    fn walk(v: &serde_json::Value, out: &mut BTreeSet<String>) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, x) in m {
                    match (k.as_str(), x) {
                        ("family", serde_json::Value::String(s)) if !s.trim().is_empty() => {
                            out.insert(s.trim().to_string());
                        }
                        _ => walk(x, out),
                    }
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    walk(&doc, &mut out);
    out.into_iter().collect()
}

/// Whether PressCraft has this font file in its list: it was in one of the folders PressCraft reads
/// (the file itself, or FontCraft's copy in ~/Library/Fonts/FontCraft) before PressCraft opened.
/// True when PressCraft isn't open (it reads everything when it opens) and outside macOS.
pub fn read_when_opened(file: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::MetadataExt;
        let Some((_, opened)) = running() else { return true };
        let home = crate::paths::home();
        let dirs = [PathBuf::from("/System/Library/Fonts"), PathBuf::from("/Library/Fonts"), home.join("Library/Fonts")];
        let mut places: Vec<PathBuf> = Vec::new();
        if dirs.iter().any(|d| file.starts_with(d)) {
            places.push(file.to_path_buf());
        }
        places.extend(crate::activate::placed(file));
        // A file's status-change time is when it was put there (a hard link changes it too).
        places.iter().any(|p| std::fs::metadata(p).is_ok_and(|m| (m.ctime() as f64 + m.ctime_nsec() as f64 / 1e9) <= opened))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = file;
        true
    }
}

/// The running PressCraft: its process id and when it opened (seconds since 1970), from the system.
#[cfg(target_os = "macos")]
fn running() -> Option<(i32, f64)> {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    let id = objc2_foundation::NSString::from_str(BUNDLE);
    let run = || unsafe {
        let list: *mut AnyObject = msg_send![class!(NSRunningApplication), runningApplicationsWithBundleIdentifier: &*id];
        let n: usize = if list.is_null() { 0 } else { msg_send![list, count] };
        if n == 0 {
            return None;
        }
        let app: *mut AnyObject = msg_send![list, objectAtIndex: 0usize];
        let pid: i32 = msg_send![app, processIdentifier];
        let date: *mut AnyObject = msg_send![app, launchDate];
        let at: f64 = if date.is_null() { 0.0 } else { msg_send![date, timeIntervalSince1970] };
        Some((pid, at))
    };
    let (pid, mut at) = objc2::exception::catch(std::panic::AssertUnwindSafe(run)).ok().flatten()?;
    // Started without Launch Services (from a terminal) it has no launch date: its process's age.
    if at <= 0.0 {
        let out = crate::paths::command("ps").args(["-o", "etime=", "-p", &pid.to_string()]).output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let (days, rest) = text.split_once('-').map(|(d, r)| (d.parse::<f64>().unwrap_or(0.0), r.to_string())).unwrap_or((0.0, text));
        let secs = rest.split(':').filter_map(|p| p.parse::<f64>().ok()).fold(0.0, |acc, v| acc * 60.0 + v) + days * 86400.0;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_secs_f64();
        at = now - secs;
    }
    Some((pid, at))
}

/// The document in front in PressCraft: its window title ("Cartel — PressCraft", "Cartel (editado)
/// — PressCraft") names it, and PressCraft's recent list has the file (opened or saved documents).
pub fn front_document() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let (pid, _) = running()?;
        let app = ax::El::app(pid)?;
        let win = app.element("AXFocusedWindow").or_else(|| app.element("AXMainWindow"))?;
        document_for_title(&win.string("AXTitle")?)
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn document_for_title(title: &str) -> Option<PathBuf> {
    let name = title.trim().trim_end_matches("PressCraft").trim_end().trim_end_matches('—').trim_end();
    let name = name.trim_end_matches("(editado)").trim();
    if name.is_empty() {
        return None;
    }
    let settings = crate::paths::home().join("Library/Application Support/PressCraft/settings.json");
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(settings).ok()?).ok()?;
    json.get("recent")?
        .as_array()?
        .iter()
        .filter_map(|p| p.as_str().map(PathBuf::from))
        .find(|p| p.file_stem().is_some_and(|s| s.to_string_lossy() == name) && p.is_file())
}

/// Whether PressCraft shows its font field now (its text tools, or text selected or being edited).
pub fn field_showing() -> bool {
    #[cfg(target_os = "macos")]
    {
        running().and_then(|(pid, _)| ax::El::app(pid)).and_then(|app| ax::font_field(&app)).is_some()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Puts `family` (and `style`, when given and PressCraft has it) on PressCraft's selected text with
/// its own font field. PressCraft must be in front (Return goes to the app in front).
pub fn set_font(family: &str, style: Option<&str>) -> Answer {
    #[cfg(target_os = "macos")]
    {
        use crate::mechanics::{PRESSCRAFT_PICK_WAIT_MS, PRESSCRAFT_STEP_MS};
        use std::time::Duration;
        let Some((pid, _)) = running() else { return Answer::Unreachable };
        let Some(app) = ax::El::app(pid) else { return Answer::Unreachable };
        let Some((field, menu)) = ax::font_field_and_menu(&app) else { return Answer::NoField };
        let step = || std::thread::sleep(Duration::from_millis(PRESSCRAFT_STEP_MS));
        let shown = |f: &ax::El| f.string("AXValue").unwrap_or_default();
        let before = shown(&field);
        // The field takes the keyboard (PressCraft opens its list and selects the name in it), then
        // the family is typed in. A space after it keeps PressCraft from reading the name it shows
        // as "nothing typed" (its list would then start at A); its search ignores the space.
        if !field.set_bool("AXFocused", true) {
            return Answer::Unreachable;
        }
        step();
        if !field.set_string("AXValue", &format!("{family} ")) {
            ax::key(53, false);
            return Answer::Unreachable;
        }
        step();
        // Return chooses the first family of PressCraft's list.
        ax::key(36, false);
        let mut now = String::new();
        let mut waited = 0;
        let mut field = field;
        while waited < PRESSCRAFT_PICK_WAIT_MS {
            std::thread::sleep(Duration::from_millis(50));
            waited += 50;
            // The same field (its widget keeps its place in AccessKit's tree); looked for again
            // only if PressCraft's interface changed under it.
            now = match field.string("AXValue") {
                Some(v) => v,
                None => match ax::font_field(&app) {
                    Some(f) => {
                        field = f;
                        shown(&field)
                    }
                    None => String::new(),
                },
            };
            if now.eq_ignore_ascii_case(family) {
                break;
            }
        }
        if !now.eq_ignore_ascii_case(family) {
            if now == format!("{family} ") {
                // Nothing of that name in PressCraft's list: its field is left as it was.
                ax::key(53, false);
            } else if !now.is_empty() && now != before {
                // Another family that starts the same way was chosen: taken back (Undo, ⌘Z).
                ax::key(6, true);
            }
            return Answer::NotInList;
        }
        // The style, from the menu beside the field (PressCraft chose the family's usual one).
        if let (Some(style), Some(menu)) = (style.filter(|s| !s.is_empty()), menu.as_ref()) {
            ax::choose_style(&app, menu, style);
        }
        Answer::Done
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (family, style);
        Answer::Unreachable
    }
}

/// Developer check (`FontCraft --test-presscraft-field [family]`) against a running PressCraft: its
/// front document, its font field as Accessibility shows it, and, with a family, that family typed
/// into the field (without Return, so nothing changes in the document).
pub fn selftest(family: Option<&str>) {
    #[cfg(target_os = "macos")]
    {
        // A PressCraft just started shows in the system's list a moment later.
        let mut found = running();
        for _ in 0..50 {
            if found.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            found = running();
        }
        let Some((pid, opened)) = found else {
            println!("PressCraft: not running");
            return;
        };
        println!("PressCraft: pid {pid}, opened at {opened:.3}; Accessibility allowed: {}", crate::autoact::trusted());
        println!("front document: {:?}", front_document());
        let Some(app) = ax::El::app(pid) else { return };
        ax::describe(&app);
        let started = std::time::Instant::now();
        let Some(field) = ax::font_field(&app) else {
            println!("font field: not showing ({} ms)", started.elapsed().as_millis());
            return;
        };
        println!("font field: value {:?} ({} ms)", field.string("AXValue"), started.elapsed().as_millis());
        if let Some(f) = family {
            println!("set value {:?}: {}", format!("{f} "), field.set_string("AXValue", &format!("{f} ")));
            std::thread::sleep(std::time::Duration::from_millis(400));
            println!("field now: {:?}", ax::font_field(&app).and_then(|f| f.string("AXValue")));
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = family;
        println!("PressCraft's field is reached through macOS Accessibility only.");
    }
}

/// Developer check (`FontCraft --test-presscraft-apply family [style]`): a running PressCraft is
/// brought to the front and the font put on its selected text, then its field and style are read.
pub fn selftest_apply(family: &str, style: Option<&str>) {
    #[cfg(target_os = "macos")]
    {
        let _ = crate::paths::command("osascript").args(["-e", &format!("tell application id \"{BUNDLE}\" to activate")]).status();
        std::thread::sleep(std::time::Duration::from_millis(400));
        let started = std::time::Instant::now();
        let answer = match set_font(family, style) {
            Answer::Done => "done",
            Answer::NotInList => "not in PressCraft's list",
            Answer::NoField => "no font field",
            Answer::Unreachable => "unreachable",
        };
        println!("set {family:?} {style:?}: {answer} in {} ms", started.elapsed().as_millis());
        if let Some(app) = running().and_then(|(pid, _)| ax::El::app(pid)) {
            println!("field now: {:?}", ax::font_field(&app).and_then(|f| f.string("AXValue")));
            ax::describe(&app);
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (family, style);
    }
}

/// Accessibility (the AX API of ApplicationServices) for PressCraft's window: AccessKit gives
/// macOS egui's widgets as elements with roles, values and placeholders.
#[cfg(target_os = "macos")]
mod ax {
    use crate::mechanics::{PRESSCRAFT_STEP_MS, PRESSCRAFT_TREE_NODES, PRESSCRAFT_TREE_TRIES};
    use core_foundation::array::{CFArrayGetCount, CFArrayGetTypeID, CFArrayGetValueAtIndex, CFArrayRef};
    use core_foundation::base::{CFGetTypeID, CFType, CFTypeRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::string::{CFString, CFStringRef};
    use std::ffi::c_void;
    use std::time::Duration;

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Pair(f64, f64);

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
        fn AXUIElementCopyAttributeValue(element: CFTypeRef, attribute: CFStringRef, value: *mut CFTypeRef) -> i32;
        fn AXUIElementCopyMultipleAttributeValues(element: CFTypeRef, attributes: CFArrayRef, options: u32, values: *mut CFArrayRef) -> i32;
        fn AXUIElementSetAttributeValue(element: CFTypeRef, attribute: CFStringRef, value: CFTypeRef) -> i32;
        fn AXUIElementPerformAction(element: CFTypeRef, action: CFStringRef) -> i32;
        fn AXUIElementCopyElementAtPosition(application: CFTypeRef, x: f32, y: f32, element: *mut CFTypeRef) -> i32;
        fn AXUIElementSetMessagingTimeout(element: CFTypeRef, seconds: f32) -> i32;
        fn AXValueGetValue(value: CFTypeRef, kind: u32, out: *mut c_void) -> u8;
        fn CGEventCreateKeyboardEvent(source: *const c_void, keycode: u16, keydown: bool) -> *mut c_void;
        fn CGEventSetFlags(event: *mut c_void, flags: u64);
        fn CGEventPost(tap: u32, event: *mut c_void);
        fn CFRelease(p: *const c_void);
    }

    /// One element of PressCraft's interface.
    #[derive(Clone)]
    pub struct El(CFType);

    /// The elements of an array value (AXChildren, AXWindows); nothing for any other value.
    fn elements_of(v: &CFType) -> Vec<El> {
        let r = v.as_CFTypeRef();
        if unsafe { CFGetTypeID(r) != CFArrayGetTypeID() } {
            return Vec::new();
        }
        let a = r as CFArrayRef;
        (0..unsafe { CFArrayGetCount(a) })
            .filter_map(|i| {
                let e = unsafe { CFArrayGetValueAtIndex(a, i) };
                (!e.is_null()).then(|| El(unsafe { CFType::wrap_under_get_rule(e as CFTypeRef) }))
            })
            .collect()
    }

    impl El {
        pub fn app(pid: i32) -> Option<El> {
            let r = unsafe { AXUIElementCreateApplication(pid) };
            if r.is_null() {
                return None;
            }
            let el = El(unsafe { CFType::wrap_under_create_rule(r) });
            // A busy PressCraft never holds FontCraft's worker for long.
            unsafe { AXUIElementSetMessagingTimeout(el.0.as_CFTypeRef(), 1.0) };
            Some(el)
        }

        fn get(&self, attribute: &str) -> Option<CFType> {
            let name = CFString::new(attribute);
            let mut v: CFTypeRef = std::ptr::null();
            let err = unsafe { AXUIElementCopyAttributeValue(self.0.as_CFTypeRef(), name.as_concrete_TypeRef(), &mut v) };
            (err == 0 && !v.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(v) })
        }

        pub fn string(&self, attribute: &str) -> Option<String> {
            self.get(attribute)?.downcast::<CFString>().map(|s| s.to_string())
        }

        pub fn element(&self, attribute: &str) -> Option<El> {
            self.get(attribute).map(El)
        }

        fn list(&self, attribute: &str) -> Vec<El> {
            self.get(attribute).map(|v| elements_of(&v)).unwrap_or_default()
        }

        /// Its role and its children in one question to PressCraft (each question waits for
        /// PressCraft's main thread, so a walk asks as few as it can).
        fn role_and_children(&self) -> (String, Vec<El>) {
            let names = core_foundation::array::CFArray::from_CFTypes(&[CFString::new("AXRole"), CFString::new("AXChildren")]);
            let mut out: CFArrayRef = std::ptr::null();
            let err = unsafe { AXUIElementCopyMultipleAttributeValues(self.0.as_CFTypeRef(), names.as_concrete_TypeRef(), 0, &mut out) };
            if err != 0 || out.is_null() {
                return (String::new(), Vec::new());
            }
            let values = unsafe { CFType::wrap_under_create_rule(out as CFTypeRef) };
            let a = values.as_CFTypeRef() as CFArrayRef;
            let at = |i: isize| -> Option<CFType> {
                (i < unsafe { CFArrayGetCount(a) }).then(|| unsafe { CFArrayGetValueAtIndex(a, i) }).filter(|p| !p.is_null()).map(|p| unsafe { CFType::wrap_under_get_rule(p as CFTypeRef) })
            };
            let role = at(0).and_then(|v| v.downcast::<CFString>()).map(|s| s.to_string()).unwrap_or_default();
            let children = at(1).map(|v| elements_of(&v)).unwrap_or_default();
            (role, children)
        }

        fn pair(&self, attribute: &str, kind: u32) -> Option<(f64, f64)> {
            let v = self.get(attribute)?;
            let mut p = Pair::default();
            (unsafe { AXValueGetValue(v.as_CFTypeRef(), kind, &mut p as *mut Pair as *mut c_void) } != 0).then_some((p.0, p.1))
        }

        /// Left, top, width and height on the screen.
        fn frame(&self) -> Option<(f64, f64, f64, f64)> {
            let (x, y) = self.pair("AXPosition", 1)?;
            let (w, h) = self.pair("AXSize", 2)?;
            Some((x, y, w, h))
        }

        pub fn set_bool(&self, attribute: &str, on: bool) -> bool {
            let name = CFString::new(attribute);
            let v = if on { CFBoolean::true_value() } else { CFBoolean::false_value() };
            unsafe { AXUIElementSetAttributeValue(self.0.as_CFTypeRef(), name.as_concrete_TypeRef(), v.as_CFTypeRef()) == 0 }
        }

        pub fn set_string(&self, attribute: &str, value: &str) -> bool {
            let name = CFString::new(attribute);
            let v = CFString::new(value);
            unsafe { AXUIElementSetAttributeValue(self.0.as_CFTypeRef(), name.as_concrete_TypeRef(), v.as_CFTypeRef()) == 0 }
        }

        /// The element of this app at a point of the screen (one question, however big its tree).
        fn at(&self, x: f64, y: f64) -> Option<El> {
            let mut e: CFTypeRef = std::ptr::null();
            let err = unsafe { AXUIElementCopyElementAtPosition(self.0.as_CFTypeRef(), x as f32, y as f32, &mut e) };
            (err == 0 && !e.is_null()).then(|| El(unsafe { CFType::wrap_under_create_rule(e) }))
        }

        fn press(&self) -> bool {
            let action = CFString::new("AXPress");
            unsafe { AXUIElementPerformAction(self.0.as_CFTypeRef(), action.as_concrete_TypeRef()) == 0 }
        }
    }

    /// Breadth first through PressCraft's windows, each element asked as it is reached (egui gives
    /// macOS a new tree whenever its interface changes, and an element of an older one no longer
    /// answers): the first one `want` takes, how many were looked at, and how many didn't answer.
    fn search(app: &El, mut want: impl FnMut(&El, &str) -> bool) -> (Option<El>, usize, usize) {
        let (mut seen, mut silent) = (0, 0);
        let mut queue: std::collections::VecDeque<El> = app.list("AXWindows").into();
        while let Some(e) = queue.pop_front() {
            if seen >= PRESSCRAFT_TREE_NODES {
                break;
            }
            seen += 1;
            let (role, children) = e.role_and_children();
            if role.is_empty() {
                silent += 1;
            }
            if want(&e, &role) {
                return (Some(e), seen, silent);
            }
            queue.extend(children);
        }
        (None, seen, silent)
    }

    /// For the developer check: how many elements PressCraft shows and which kinds.
    pub fn describe(app: &El) {
        let mut kinds: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        let (_, seen, silent) = search(app, |e, role| {
            *kinds.entry(role.to_string()).or_default() += 1;
            if matches!(role, "AXTextField" | "AXPopUpButton" | "AXCheckBox" | "AXList" | "AXMenuItem") {
                println!("  {role}: title {:?} description {:?} value {:?} frame {:?}", e.string("AXTitle"), e.string("AXDescription"), e.string("AXValue"), e.frame());
            }
            false
        });
        println!("elements: {seen} ({silent} didn't answer) {kinds:?}");
    }

    /// The text options' font field: the text field with the style menu right beside it, on its
    /// row (studio2.rs `font_picker` draws them together); its placeholder "Fuente" only shows
    /// while it is empty (AccessKit gives a placeholder for empty fields only). AccessKit hands
    /// PressCraft's widgets to macOS only after the first question, and an interface that just
    /// changed may not answer, so it is looked for a few times. With it, its style menu.
    pub fn font_field_and_menu(app: &El) -> Option<(El, Option<El>)> {
        type Frame = (f64, f64, f64, f64);
        for attempt in 0..PRESSCRAFT_TREE_TRIES {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(250));
            }
            let mut fields: Vec<(El, Option<String>, Frame)> = Vec::new();
            let mut menus: Vec<(El, Frame)> = Vec::new();
            let (_, seen, silent) = search(app, |e, role| {
                match role {
                    "AXTextField" => {
                        if let Some(f) = e.frame() {
                            fields.push((e.clone(), e.string("AXPlaceholderValue"), f));
                        }
                    }
                    "AXPopUpButton" => {
                        if let Some(f) = e.frame() {
                            menus.push((e.clone(), f));
                        }
                    }
                    _ => {}
                }
                false
            });
            let beside = |(x, y, w, h): Frame| {
                menus.iter().find(|(_, (mx, my, _, mh))| *mx >= x + w - 1.0 && *mx - (x + w) < 60.0 && ((my + mh / 2.0) - (y + h / 2.0)).abs() < h.max(*mh)).map(|(m, _)| m.clone())
            };
            for (e, placeholder, f) in fields {
                let menu = beside(f);
                if menu.is_some() || placeholder.as_deref() == Some(super::FONT_FIELD) {
                    return Some((e, menu));
                }
            }
            // A whole interface, every element answering, without the field: it isn't showing.
            if seen > 40 && silent == 0 {
                return None;
            }
        }
        None
    }

    pub fn font_field(app: &El) -> Option<El> {
        font_field_and_menu(app).map(|(f, _)| f)
    }

    /// The style menu beside the font field is opened and `style` chosen in it, when it lists it;
    /// the list is closed again either way. The list opens right below the menu, so its entries are
    /// found by asking what is at points down from there (one question each, not a whole walk).
    pub fn choose_style(app: &El, menu: &El, style: &str) {
        let pause = || std::thread::sleep(Duration::from_millis(PRESSCRAFT_STEP_MS));
        let Some((cx, cy, cw, ch)) = menu.frame() else { return };
        if menu.string("AXValue").is_some_and(|v| v.trim().eq_ignore_ascii_case(style)) {
            return;
        }
        // The list's entries: below the menu, starting at its left edge, no wider than a little more.
        let entries = || -> Vec<(El, String)> {
            let mut out: Vec<(El, String)> = Vec::new();
            let x = cx + (cw / 2.0).min(20.0);
            let mut y = cy + ch + 3.0;
            while y < cy + ch + 420.0 {
                if let Some(e) = app.at(x, y) {
                    let ok = matches!(e.string("AXRole").as_deref(), Some("AXCheckBox" | "AXButton" | "AXRadioButton" | "AXMenuItem"))
                        && e.frame().is_some_and(|(ex, ey, ew, _)| ey > cy && ex >= cx - 2.0 && ex + ew <= cx + cw + 30.0);
                    if ok {
                        let name = e.string("AXTitle").or_else(|| e.string("AXDescription")).unwrap_or_default();
                        if !out.iter().any(|(_, n)| *n == name) {
                            out.push((e, name));
                        }
                    }
                }
                y += 9.0;
            }
            out
        };
        if !menu.press() {
            return;
        }
        let mut waited = 0;
        loop {
            std::thread::sleep(Duration::from_millis(120));
            waited += 120;
            let list = entries();
            if let Some((e, _)) = list.iter().find(|(_, n)| n.trim().eq_ignore_ascii_case(style)) {
                e.press();
                pause();
            }
            if !list.is_empty() {
                // Still open (a click from Accessibility isn't a pointer click, which is what closes
                // it): the menu closes it.
                if !entries().is_empty() {
                    menu.press();
                }
                return;
            }
            if waited >= crate::mechanics::PRESSCRAFT_PICK_WAIT_MS {
                return;
            }
        }
    }

    /// One key (macOS key code) pressed and released in the app in front, with ⌘ when `command`.
    pub fn key(code: u16, command: bool) {
        for down in [true, false] {
            let ev = unsafe { CGEventCreateKeyboardEvent(std::ptr::null(), code, down) };
            if ev.is_null() {
                return;
            }
            unsafe {
                if command {
                    // kCGEventFlagMaskCommand
                    CGEventSetFlags(ev, 0x0010_0000);
                }
                // kCGHIDEventTap: as if typed on the keyboard.
                CGEventPost(0, ev);
                CFRelease(ev);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn families_from_a_presscraft_pdf() {
        use std::io::Write;
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        z.start_file("mimetype", stored).unwrap();
        z.write_all(b"application/x-presscraft").unwrap();
        z.start_file("document.json", zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(br#"{"stories":[{"paras":[{"runs":[{"fmt":{"family":"Lora","size":12}}]}]}],"char_styles":[{"name":"T","family":"Roboto Slab"},{"name":"U","family":null}],"tables":[{"family":"Lora"}]}"#).unwrap();
        let zip = z.finish().unwrap().into_inner();
        let mut pdf = b"%PDF-1.7\n1 0 obj\n<< /Type /EmbeddedFile /Length 9 >>\nstream\n".to_vec();
        pdf.extend_from_slice(&zip);
        pdf.extend_from_slice(b"\nendstream\nendobj\n%%EOF\n");
        assert_eq!(super::families(&pdf), vec!["Lora".to_string(), "Roboto Slab".to_string()]);
        assert_eq!(super::families(&zip), vec!["Lora".to_string(), "Roboto Slab".to_string()]);
        assert!(super::families(b"%PDF-1.4 no PressCraft here").is_empty());
    }

    #[test]
    fn title_names_the_document() {
        // No recent list here: only the name parsing is checked (an untitled one names nothing).
        assert!(super::document_for_title("PressCraft").is_none());
    }
}
