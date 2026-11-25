//! FontCraft's own library format and its folder.
//!
//! - A FontCraft library is `~/FontCraft/<name>.fcl`: a container folder (like a macOS package)
//!   holding `library.json` (format, version, name, creation time) and the fonts, sorted into one
//!   folder per first letter of the family name: `A/`, `B/`, … `0-9/`, `#/`.
//! - `~/FontCraft` is made when the first library is created.
//! - The folder can be shared with every user of the computer or with one user (Settings ›
//!   General); FontCraft then also shows the libraries other users share with this one.
//! - Anything that needs a system permission opens the system's own security settings.

use crate::libs::LibKind;
use std::path::{Path, PathBuf};

pub const EXT: &str = "fcl";

/// `~/FontCraft` of the person using FontCraft.
pub fn home_dir() -> PathBuf {
    crate::paths::home().join("FontCraft")
}

fn is_font(p: &Path) -> bool {
    matches!(
        p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(),
        Some("ttf" | "otf" | "ttc" | "otc")
    )
}

/// A name that is safe as a file name.
fn safe_name(name: &str) -> String {
    let s: String = name.trim().chars().map(|c| if matches!(c, '/' | ':' | '\\' | '\0') { '-' } else { c }).collect();
    let s = s.trim_start_matches('.').to_string();
    if s.is_empty() { "Library".into() } else { s }
}

/// Makes `~/FontCraft/<name>.fcl` (and `~/FontCraft` if it isn't there yet).
pub fn create(name: &str) -> Result<PathBuf, String> {
    let dir = home_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.{EXT}", safe_name(name)));
    if path.exists() {
        return Err(format!("{} already exists", path.display()));
    }
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    let created = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let manifest = serde_json::json!({
        "format": "fontcraft-library",
        "version": 1,
        "name": name.trim(),
        "created": created,
        "layout": "first-letter-folders",
    });
    std::fs::write(path.join("library.json"), serde_json::to_string_pretty(&manifest).unwrap_or_default()).map_err(|e| e.to_string())?;
    mark_as_library(&path);
    Ok(path)
}

/// The family name a font file declares (typographic family first), or its file name.
pub fn family_name(path: &Path) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
    let Ok(f) = std::fs::File::open(path) else { return stem };
    let Ok(map) = (unsafe { memmap2::Mmap::map(&f) }) else { return stem };
    let Ok(face) = ttf_parser::Face::parse(&map, 0) else { return stem };
    for id in [ttf_parser::name_id::TYPOGRAPHIC_FAMILY, ttf_parser::name_id::FAMILY] {
        let mut any = None;
        for n in face.names() {
            if n.name_id == id {
                if let Some(s) = n.to_string() {
                    if n.language() == ttf_parser::Language::English_UnitedStates {
                        return s;
                    }
                    any.get_or_insert(s);
                }
            }
        }
        if let Some(s) = any {
            return s;
        }
    }
    stem
}

/// The folder a family goes into: its first letter ("A"…"Z", also "Ñ", "Ö"…), "0-9" or "#".
pub fn letter_for(name: &str) -> String {
    let Some(c) = name.trim().chars().next() else { return "#".into() };
    if c.is_ascii_digit() {
        "0-9".into()
    } else if c.is_alphabetic() {
        c.to_uppercase().collect()
    } else {
        "#".into()
    }
}

/// Every font in the chosen files and folders (folders are searched all the way down).
pub fn expand(picked: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in picked {
        if p.is_dir() {
            let mut found = Vec::new();
            crate::fonts::walk(p, &mut found);
            out.extend(found.into_iter().filter(|f| is_font(f)));
        } else if is_font(p) {
            out.push(p.clone());
        }
    }
    out.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    out.dedup();
    out
}

/// Exports fonts as RightFont does (-[RFFontService exportFonts:toLocation:], 0x10007e218): a
/// folder named after the font (or family) is made in `dest` with its parents, and each file is
/// copied into it; a file already there keeps its place. Returns how many were copied and where.
#[cfg_attr(any(target_os = "android", target_os = "ios"), allow(dead_code))]
pub fn export_fonts(dest: &Path, name: &str, files: &[PathBuf]) -> Result<(usize, PathBuf), String> {
    let dir = dest.join(safe_name(name));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut n = 0;
    for f in files {
        let Some(file_name) = f.file_name() else { continue };
        let to = dir.join(file_name);
        if to.exists() {
            continue;
        }
        if std::fs::copy(f, &to).is_ok() {
            n += 1;
        }
    }
    Ok((n, dir))
}

/// Copies fonts into a library: a FontCraft library puts each one in its letter folder, a plain
/// folder gets them at its top level. Files already there are left alone. Returns how many were copied.
pub fn add_fonts(lib: &Path, kind: LibKind, picked: &[PathBuf]) -> usize {
    let mut copied = 0;
    // A font the library already has (same PostScript name, even under another file name) is
    // skipped, as RightFont does when importing (-[RFFontFileHandler existingFontMatchingFont:],
    // 0x1000ff100, looks fonts up by name).
    let mut have: std::collections::HashSet<String> = {
        let mut inside = Vec::new();
        crate::fonts::walk(lib, &mut inside);
        inside.iter().filter(|p| is_font(p)).filter_map(|p| postscript_name(p)).collect()
    };
    for f in expand(picked) {
        let Some(file) = f.file_name() else { continue };
        let ps = postscript_name(&f);
        if ps.as_ref().is_some_and(|n| have.contains(n)) {
            continue;
        }
        let dir = match kind {
            LibKind::Fcl => lib.join(letter_for(&family_name(&f))),
            LibKind::Folder => lib.to_path_buf(),
        };
        if f.starts_with(lib) {
            continue;
        }
        let dest = dir.join(file);
        if dest.exists() || std::fs::create_dir_all(&dir).is_err() {
            continue;
        }
        if std::fs::copy(&f, &dest).is_ok() {
            copied += 1;
            if let Some(n) = ps {
                have.insert(n);
            }
        }
    }
    copied
}

/// A font file's PostScript name (name ID 6 of its first face).
pub fn postscript_name(path: &Path) -> Option<String> {
    let f = std::fs::File::open(path).ok()?;
    let map = unsafe { memmap2::Mmap::map(&f) }.ok()?;
    let face = ttf_parser::Face::parse(&map, 0).ok()?;
    face.names().into_iter().filter(|n| n.name_id == ttf_parser::name_id::POST_SCRIPT_NAME).find_map(|n| n.to_string())
}

/// The system's own Open panel, allowing fonts and folders together (several at once).
#[cfg(target_os = "macos")]
pub fn pick_fonts_or_folders() -> Vec<PathBuf> {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    let mut out = Vec::new();
    unsafe {
        let panel: Retained<AnyObject> = msg_send![class!(NSOpenPanel), openPanel];
        let _: () = msg_send![&panel, setCanChooseFiles: true];
        let _: () = msg_send![&panel, setCanChooseDirectories: true];
        let _: () = msg_send![&panel, setAllowsMultipleSelection: true];
        let answer: isize = msg_send![&panel, runModal];
        if answer != 1 {
            return out;
        }
        let urls: Retained<AnyObject> = msg_send![&panel, URLs];
        let n: usize = msg_send![&urls, count];
        for i in 0..n {
            let url: *mut AnyObject = msg_send![&urls, objectAtIndex: i];
            if url.is_null() {
                continue;
            }
            let path: *mut AnyObject = msg_send![url, path];
            if path.is_null() {
                continue;
            }
            let c: *const std::os::raw::c_char = msg_send![path, UTF8String];
            if !c.is_null() {
                out.push(PathBuf::from(std::ffi::CStr::from_ptr(c).to_string_lossy().to_string()));
            }
        }
    }
    out
}

#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn pick_fonts_or_folders() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(all(not(target_os = "macos"), not(target_os = "android"), not(target_os = "ios")))]
pub fn pick_fonts_or_folders() -> Vec<PathBuf> {
    let mut v = rfd::FileDialog::new().add_filter("Fonts", &["ttf", "otf", "ttc", "otc"]).pick_files().unwrap_or_default();
    if v.is_empty() {
        v = rfd::FileDialog::new().pick_folders().unwrap_or_default();
    }
    v
}

/// The system's folder/library picker. On macOS, uses NSOpenPanel beginWithCompletionHandler:
/// which runs completely asynchronously without modal runloop reentrancy, preventing winit SIGABRT.
/// On other platforms, executes on a background thread.
#[cfg(target_os = "macos")]
pub fn pick_folder_or_library_async(start: Option<PathBuf>, cb: impl FnOnce(Option<PathBuf>) + Send + 'static) {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use block2::RcBlock;

    let run = || unsafe {
        let panel: Retained<AnyObject> = msg_send![class!(NSOpenPanel), openPanel];
        let _: () = msg_send![&panel, setCanChooseFiles: true];
        let _: () = msg_send![&panel, setCanChooseDirectories: true];
        let _: () = msg_send![&panel, setAllowsMultipleSelection: false];
        let _: () = msg_send![&panel, setCanCreateDirectories: true];
        let _: () = msg_send![&panel, setTreatsFilePackagesAsDirectories: false];
        if let Some(dir) = start.as_deref().filter(|d| d.is_dir()) {
            let ns = objc2_foundation::NSString::from_str(&dir.to_string_lossy());
            let url: *mut AnyObject = msg_send![class!(NSURL), fileURLWithPath: &*ns, isDirectory: true];
            if !url.is_null() {
                let _: () = msg_send![&panel, setDirectoryURL: url];
            }
        }
        let panel_clone = panel.clone();
        let cb_cell = std::sync::Mutex::new(Some(cb));
        let block = RcBlock::new(move |res: isize| {
            let mut path_opt = None;
            if res == 1 {
                let url: *mut AnyObject = msg_send![&*panel_clone, URL];
                if !url.is_null() {
                    let path: *mut AnyObject = msg_send![url, path];
                    if !path.is_null() {
                        let c: *const std::os::raw::c_char = msg_send![path, UTF8String];
                        if !c.is_null() {
                            let p = PathBuf::from(std::ffi::CStr::from_ptr(c).to_string_lossy().to_string());
                            if p.is_dir() {
                                path_opt = Some(p);
                            }
                        }
                    }
                }
            }
            if let Ok(mut lock) = cb_cell.lock() {
                if let Some(f) = lock.take() {
                    f(path_opt);
                }
            }
        });
        let _: () = msg_send![&*panel, beginWithCompletionHandler: &*block];
    };
    let _ = objc2::exception::catch(std::panic::AssertUnwindSafe(run));
}

#[cfg(all(not(target_os = "macos"), not(target_os = "android"), not(target_os = "ios")))]
pub fn pick_folder_or_library_async(start: Option<PathBuf>, cb: impl FnOnce(Option<PathBuf>) + Send + 'static) {
    std::thread::spawn(move || {
        let mut dlg = rfd::FileDialog::new();
        if let Some(dir) = start.as_deref().filter(|d| d.is_dir()) {
            dlg = dlg.set_directory(dir);
        }
        let res = dlg.pick_folder();
        cb(res);
    });
}

#[cfg(any(target_os = "android", target_os = "ios"))]
#[allow(dead_code)]
pub fn pick_folder_or_library_async(_start: Option<PathBuf>, cb: impl FnOnce(Option<PathBuf>) + Send + 'static) {
    cb(None);
}

/// Fallback synchronous folder picker (safe on non-macOS platforms).
#[cfg(all(not(target_os = "android"), not(target_os = "ios")))]
#[allow(dead_code)]
pub fn pick_folder_or_library(start: Option<&Path>) -> Option<PathBuf> {
    let mut dlg = rfd::FileDialog::new();
    if let Some(dir) = start.filter(|d| d.is_dir()) {
        dlg = dlg.set_directory(dir);
    }
    dlg.pick_folder()
}

/// On mobile (Android, iOS) libraries are accessed via sandbox paths.
#[cfg(any(target_os = "android", target_os = "ios"))]
#[allow(dead_code)]
pub fn pick_folder_or_library(_start: Option<&Path>) -> Option<PathBuf> {
    None
}

/// Marks a FontCraft library folder as FontCraft's on systems where it is a plain folder, so the
/// file manager shows it with FontCraft's library icon: Windows `desktop.ini` (the folder is made a
/// "system" folder so Explorer reads it), Linux the GNOME/Nautilus custom icon and a KDE `.directory`.
/// On macOS the library is a package that Finder already shows as FontCraft's document.
pub fn mark_as_library(dir: &Path) {
    if !dir.is_dir() || dir.extension().and_then(|e| e.to_str()) != Some(EXT) {
        return;
    }
    #[cfg(target_os = "windows")]
    {
        let ini = dir.join("desktop.ini");
        let icon = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("fontcraft-library.ico")));
        if let Some(icon) = icon.filter(|i| i.exists()) {
            if !ini.exists() {
                let text = format!("[.ShellClassInfo]\r\nIconResource={},0\r\nInfoTip=FontCraft Library\r\n", icon.display());
                if std::fs::write(&ini, text).is_ok() {
                    let _ = crate::paths::command("attrib").args(["+h", "+s"]).arg(&ini).status();
                    let _ = crate::paths::command("attrib").arg("+s").arg(dir).status();
                }
            }
        }
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
    {
        let dot = dir.join(".directory");
        if !dot.exists() {
            let _ = std::fs::write(&dot, "[Desktop Entry]\nIcon=application-x-fontcraft-library\nComment=FontCraft Library\n");
        }
        let icon = ["/usr/share/icons/hicolor/256x256/mimetypes/application-x-fontcraft-library.png"]
            .into_iter()
            .map(std::path::PathBuf::from)
            .chain(std::iter::once(crate::paths::home().join(".local/share/icons/hicolor/256x256/mimetypes/application-x-fontcraft-library.png")))
            .find(|p| p.exists());
        if let Some(icon) = icon {
            let _ = crate::paths::command("gio").args(["set"]).arg(dir).arg("metadata::custom-icon").arg(format!("file://{}", icon.display())).status();
        }
    }
}

/// The other people with an account on this computer (not system accounts).
pub fn other_users() -> Vec<String> {
    let me = std::env::var("USER").unwrap_or_default();
    let mut v = Vec::new();
    if cfg!(target_os = "macos") {
        if let Ok(o) = crate::paths::command("dscl").args([".", "list", "/Users", "UniqueID"]).output() {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                let mut it = line.split_whitespace();
                let (Some(name), Some(uid)) = (it.next(), it.next()) else { continue };
                // People have a home folder under /Users (service accounts such as "macports" don't).
                let has_home = Path::new("/Users").join(name).is_dir();
                if uid.parse::<i64>().is_ok_and(|u| u >= 501) && !name.starts_with('_') && name != me && has_home {
                    v.push(name.to_string());
                }
            }
        }
    } else if let Ok(text) = std::fs::read_to_string("/etc/passwd") {
        for line in text.lines() {
            let f: Vec<&str> = line.split(':').collect();
            if f.len() > 2 && f[2].parse::<i64>().is_ok_and(|u| u >= 1000 && u < 60000) && f[0] != me {
                v.push(f[0].to_string());
            }
        }
    }
    v.sort();
    v
}

/// Who `~/FontCraft` is shared with.
#[derive(Clone, PartialEq, Debug)]
pub enum ShareWith {
    Everyone,
    User(String),
}

impl ShareWith {
    pub fn to_setting(&self) -> String {
        match self {
            ShareWith::Everyone => "all".into(),
            ShareWith::User(u) => format!("user:{u}"),
        }
    }
    pub fn from_setting(s: &str) -> Option<ShareWith> {
        match s {
            "all" => Some(ShareWith::Everyone),
            _ => s.strip_prefix("user:").filter(|u| !u.is_empty()).map(|u| ShareWith::User(u.to_string())),
        }
    }
    fn principal(&self) -> String {
        match self {
            ShareWith::Everyone => "group:everyone".into(),
            ShareWith::User(u) => format!("user:{u}"),
        }
    }
}

fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    let o = crate::paths::command(cmd).args(args).output().map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

/// Shares `dir` (normally `~/FontCraft`) with everyone or with one user: they can open, add and
/// change its libraries. Replaces any earlier sharing of `dir`. With `home`, the home folder also
/// lets them pass through to it (only that: they can't list or read anything else there).
pub fn share(dir: &Path, with: &ShareWith, before: Option<&ShareWith>, home: Option<&Path>) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let d = dir.to_string_lossy().to_string();
    if cfg!(target_os = "macos") {
        // Earlier sharing of this folder goes (FontCraft made the folder, so its access list is its own).
        run("chmod", &["-R", "-N", &d])?;
        let rights = "list,add_file,search,add_subdirectory,delete_child,readattr,writeattr,readextattr,writeextattr,readsecurity,read,write,append,execute,delete,file_inherit,directory_inherit";
        run("chmod", &["-R", "+a", &format!("{} allow {rights}", with.principal()), &d])?;
        if let Some(h) = home {
            let h = h.to_string_lossy().to_string();
            if let Some(old) = before.filter(|o| *o != with) {
                let _ = run("chmod", &["-a", &format!("{} allow search", old.principal()), &h]);
            }
            let entry = format!("{} allow search", with.principal());
            let listed = crate::paths::command("ls").args(["-led", &h]).output().map(|o| String::from_utf8_lossy(&o.stdout).contains(&entry)).unwrap_or(false);
            if !listed {
                run("chmod", &["+a", &entry, &h])?;
            }
        }
        Ok(())
    } else if cfg!(target_os = "linux") {
        match with {
            ShareWith::Everyone => run("chmod", &["-R", "a+rwX", &d]),
            ShareWith::User(u) => run("setfacl", &["-R", "-m", &format!("u:{u}:rwX,d:u:{u}:rwX"), &d]),
        }
    } else {
        let who = match with {
            ShareWith::Everyone => "Everyone".to_string(),
            ShareWith::User(u) => u.clone(),
        };
        run("icacls", &[&d, "/grant", &format!("{who}:(OI)(CI)M"), "/T"])
    }
}

/// FontCraft libraries other users share with this one (`/Users/<name>/FontCraft/*.fcl` that can be read).
pub fn shared_with_me() -> Vec<(String, PathBuf)> {
    let me = crate::paths::home();
    let base = if cfg!(target_os = "macos") { "/Users" } else { "/home" };
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(base) else { return out };
    for e in rd.flatten() {
        let home = e.path();
        if home == me {
            continue;
        }
        let user = e.file_name().to_string_lossy().to_string();
        let Ok(libs) = std::fs::read_dir(home.join("FontCraft")) else { continue };
        for l in libs.flatten() {
            let p = l.path();
            if p.extension().is_some_and(|x| x == EXT) && p.is_dir() {
                out.push((format!("{} ({user})", crate::libscan::library_name(&p)), p));
            }
        }
    }
    out.sort();
    out
}

/// Which of the system's security settings to open.
#[derive(Clone, Copy)]
pub enum Pane {
    /// Control other apps (keystrokes, the selected text).
    Accessibility,
    /// Send AppleScript commands to other apps.
    Automation,
    /// Reach every folder.
    Files,
}

/// Opens the system's own security settings at the right place (and, for Accessibility,
/// shows the system's request to allow FontCraft).
pub fn open_security(pane: Pane) {
    #[cfg(target_os = "macos")]
    {
        // The system's request only when FontCraft isn't allowed yet; once it is, the settings
        // page simply opens.
        if let Pane::Accessibility = pane {
            if !crate::autoact::trusted() {
                ask_accessibility();
            }
        }
        let anchor = match pane {
            Pane::Accessibility => "Privacy_Accessibility",
            Pane::Automation => "Privacy_Automation",
            Pane::Files => "Privacy_AllFiles",
        };
        let _ = crate::paths::command("open").arg(format!("x-apple.systempreferences:com.apple.preference.security?{anchor}")).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = pane;
        let _ = crate::paths::command("cmd").args(["/C", "start", "ms-settings:privacy"]).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = pane;
    }
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: core_foundation::dictionary::CFDictionaryRef) -> u8;
}

/// AXIsProcessTrustedWithOptions({kAXTrustedCheckOptionPrompt: true}), as RightFont calls it:
/// whether FontCraft is allowed, and when it isn't, macOS's own "allow FontCraft to control this
/// computer" request.
#[cfg(target_os = "macos")]
pub fn ask_accessibility() -> bool {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;
    let dict = CFDictionary::from_CFType_pairs(&[(CFString::new("AXTrustedCheckOptionPrompt").as_CFType(), CFBoolean::true_value().as_CFType())]);
    unsafe { AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef()) != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters() {
        assert_eq!(letter_for("Avenir"), "A");
        assert_eq!(letter_for("ñandú"), "Ñ");
        assert_eq!(letter_for("3270 Font"), "0-9");
        assert_eq!(letter_for("@Symbols"), "#");
    }

    #[test]
    fn fonts_go_into_letter_folders() {
        let lib = std::env::temp_dir().join(format!("fontcraft-test-{}.fcl", std::process::id()));
        let _ = std::fs::create_dir_all(&lib);
        let src = PathBuf::from("/System/Library/Fonts/Supplemental/Arial.ttf");
        if src.exists() {
            assert_eq!(add_fonts(&lib, LibKind::Fcl, &[src.clone()]), 1);
            assert!(lib.join("A").join("Arial.ttf").exists());
            // Already there: not copied again.
            assert_eq!(add_fonts(&lib, LibKind::Fcl, &[src]), 0);
            let pv = crate::libscan::preview(&lib);
            assert_eq!(pv.subs.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["A"]);
            // In the sidebar the library lists its families by full name, not the letter folders.
            let sets = crate::libs::sets_for(&lib, LibKind::Fcl);
            assert_eq!(sets.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["Arial"]);
            assert_eq!(sets[0].files, vec!["Arial.ttf".to_string()]);
        }
        let _ = std::fs::remove_dir_all(&lib);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn share_sets_access_list() {
        let dir = std::env::temp_dir().join(format!("fontcraft-share-{}", std::process::id()));
        let me = ShareWith::User(std::env::var("USER").unwrap_or_default());
        share(&dir, &me, None, None).unwrap();
        let out = crate::paths::command("ls").args(["-led", &dir.to_string_lossy()]).output().unwrap();
        assert!(String::from_utf8_lossy(&out.stdout).contains("allow list"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn share_setting_round_trip() {
        for s in [ShareWith::Everyone, ShareWith::User("ana".into())] {
            assert_eq!(ShareWith::from_setting(&s.to_setting()), Some(s));
        }
        assert_eq!(ShareWith::from_setting(""), None);
    }
}
