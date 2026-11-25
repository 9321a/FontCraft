//! Turning a font file on or off for every app (the font file stays where it is;
//! macOS is told to load or unload it).
//!
//! RightFont registers fonts with the persistent scope (2, -[RFFontActivator
//! activateFontByName:andURL:andEnabled:] at 0x10004b974), so they stay on in the system after it
//! quits and after logging out. The person asked that FontCraft never leave fonts on in the system:
//! FontCraft uses the session scope (3) instead, so nothing outlives the login session even if
//! FontCraft is stopped abruptly, and it turns every font it turned on off again when the document
//! closes, the app quits or FontCraft quits (`App::autoact_quit`); fonts the person keeps on are
//! turned on again the next time FontCraft opens. A font already registered with another scope
//! (persistent, from earlier versions) is unregistered from it first; turning off unregisters with
//! whatever scope the font has.
//!
//! macOS apps that read the font folders themselves instead of asking CoreText (the ArtCraft apps,
//! which use fontdb; Inkscape, Scribus, GIMP and Krita, which use fontconfig) never see a font only
//! registered with CoreText, which is all RightFont does. So while a font is on, FontCraft also
//! keeps its file in ~/Library/Fonts/FontCraft (a hard link on the same disk, else a copy: those
//! apps skip symbolic links, and so does macOS) and registers that file, not the original: one
//! registration (no duplicate in the font menus), at once for CoreText apps, and in a folder every
//! other app reads. Turning off unregisters it and takes the file out; at start, anything a crash
//! left there is taken out, so nothing stays on.

use std::path::Path;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod mac {
    use core_foundation::base::TCFType;
    use core_foundation::error::{CFError, CFErrorRef};
    use core_foundation::url::{CFURLRef, CFURL};
    use std::path::Path;

    /// kCTFontManagerScopeSession: on until FontCraft turns it off, at the latest until logout.
    #[allow(dead_code)]
    const SCOPE_SESSION: u32 = 3;
    /// kCTFontManagerScopePersistent (RightFont's, 0x10004b974): what earlier FontCraft versions used.
    const SCOPE_PERSISTENT: u32 = 2;
    #[allow(dead_code)]
    const SCOPE_NONE: u32 = 0;
    const ALREADY_REGISTERED: isize = 105;
    const NOT_REGISTERED: isize = 201;

    #[link(name = "CoreText", kind = "framework")]
    extern "C" {
        fn CTFontManagerRegisterFontsForURL(url: CFURLRef, scope: u32, error: *mut CFErrorRef) -> u8;
        fn CTFontManagerUnregisterFontsForURL(url: CFURLRef, scope: u32, error: *mut CFErrorRef) -> u8;
        #[cfg(target_os = "macos")]
        fn CTFontManagerGetScopeForURL(url: CFURLRef) -> u32;
        #[cfg(target_os = "macos")]
        fn CTFontManagerCreateFontDescriptorsFromURL(url: *const std::ffi::c_void) -> *const std::ffi::c_void;
        #[cfg(target_os = "macos")]
        fn CTFontManagerEnableFontDescriptors(descriptors: *const std::ffi::c_void, enable: u8);
    }

    fn error_code(err: CFErrorRef) -> Option<isize> {
        if err.is_null() {
            None
        } else {
            Some(unsafe { CFError::wrap_under_create_rule(err) }.code())
        }
    }

    /// User fonts folder: ~/Library/Fonts/FontCraft on macOS; Documents/Fonts on iOS.
    pub fn folder() -> std::path::PathBuf {
        #[cfg(target_os = "macos")]
        {
            crate::paths::home().join("Library/Fonts/FontCraft")
        }
        #[cfg(target_os = "ios")]
        {
            crate::paths::home().join("Documents/Fonts")
        }
    }

    pub fn set(path: &Path, on: bool) -> bool {
        let dir = folder();
        if on {
            // The original is registered with CoreText right away, as RightFont does (0x10004b974)
            // and as FontCraft did when Affinity and the Apple apps took the font at once; a link
            // in the user's font folder is left beside it for the apps that only read the font
            // folders (ArtCraft's fontdb, fontconfig apps), which macOS also picks up by itself.
            super::place_in(&dir, path);
            return register(path, true);
        }
        // Off: the placed file and the original, whichever is registered.
        let mut ok = true;
        if let Some(placed) = super::slot(&dir, path).filter(|p| p.exists()) {
            ok = register(&placed, false);
            let _ = std::fs::remove_file(&placed);
        }
        register(path, false) && ok
    }

    /// Everything in ~/Library/Fonts/FontCraft that isn't one of `keep` (by file name and size)
    /// goes: unregistered and taken out (what a crash or an earlier session left there).
    pub fn sweep(keep: &[std::path::PathBuf]) {
        let Ok(rd) = std::fs::read_dir(folder()) else { return };
        let wanted: Vec<(std::ffi::OsString, u64)> = keep.iter().filter_map(|p| Some((p.file_name()?.to_os_string(), std::fs::metadata(p).ok()?.len()))).collect();
        for e in rd.flatten() {
            let p = e.path();
            let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            if wanted.iter().any(|(n, s)| *n == e.file_name() && *s == size) {
                continue;
            }
            register(&p, false);
            let _ = std::fs::remove_file(&p);
        }
    }

    /// Registers or unregisters one file with CoreText.
    fn register(path: &Path, on: bool) -> bool {
        let Some(url) = CFURL::from_path(path, false) else { return false };
        let u = url.as_concrete_TypeRef();
        let mut err: CFErrorRef = std::ptr::null_mut();
        #[cfg(target_os = "macos")]
        {
            let current = unsafe { CTFontManagerGetScopeForURL(u) };
            if on {
                if current == SCOPE_SESSION {
                    return true;
                }
                if current != SCOPE_NONE {
                    // Registered some other way: take it off that scope first, as RightFont does.
                    unsafe { CTFontManagerUnregisterFontsForURL(u, current, std::ptr::null_mut()) };
                }
                let ok = unsafe { CTFontManagerRegisterFontsForURL(u, SCOPE_SESSION, &mut err) } != 0;
                let ok = ok || error_code(err) == Some(ALREADY_REGISTERED);
                if ok {
                    unsafe {
                        let descs = CTFontManagerCreateFontDescriptorsFromURL(u as *const std::ffi::c_void);
                        if !descs.is_null() {
                            CTFontManagerEnableFontDescriptors(descs, 1);
                            core_foundation::base::CFRelease(descs);
                        }
                    }
                }
                ok
            } else {
                if current == SCOPE_NONE {
                    return true;
                }
                let ok = unsafe { CTFontManagerUnregisterFontsForURL(u, current, &mut err) } != 0;
                ok || error_code(err) == Some(NOT_REGISTERED)
            }
        }
        #[cfg(target_os = "ios")]
        {
            if on {
                let ok = unsafe { CTFontManagerRegisterFontsForURL(u, SCOPE_PERSISTENT, &mut err) } != 0;
                ok || error_code(err) == Some(ALREADY_REGISTERED)
            } else {
                let ok = unsafe { CTFontManagerUnregisterFontsForURL(u, SCOPE_PERSISTENT, &mut err) } != 0;
                ok || error_code(err) == Some(NOT_REGISTERED)
            }
        }
    }

    /// Whether a font file is still registered for good (persistent scope, from earlier versions).
    pub fn persistent(path: &Path) -> bool {
        #[cfg(target_os = "macos")]
        {
            let Some(url) = CFURL::from_path(path, false) else { return false };
            unsafe { CTFontManagerGetScopeForURL(url.as_concrete_TypeRef()) == SCOPE_PERSISTENT }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = path;
            false
        }
    }
}

/// Fonts earlier versions left on for good in the system (macOS persistent scope) are taken off;
/// FontCraft turns on what it should itself. Other systems keep nothing on after a session.
pub fn clear_leftovers(paths: &[std::path::PathBuf]) {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    for p in paths {
        if mac::persistent(p) {
            mac::set(p, false);
        }
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "ios")))]
    let _ = paths;
}

/// The file FontCraft keeps in ~/Library/Fonts/FontCraft while this font is on (macOS), if it is
/// there: apps that read the font folders themselves (PressCraft) see the font through it.
#[cfg(target_os = "macos")]
pub fn placed(path: &Path) -> Option<std::path::PathBuf> {
    slot(&mac::folder(), path).filter(|p| p.exists())
}

/// At start, before the fonts kept on are turned on again: what an earlier session or a crash left
/// in FontCraft's user font folder (macOS/Android) goes, except the files of `keep`.
pub fn sweep(keep: &[std::path::PathBuf]) {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    mac::sweep(keep);
    #[cfg(target_os = "android")]
    android::sweep(keep);
    #[cfg(all(not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android")))]
    let _ = keep;
}

#[cfg(target_os = "android")]
mod android {
    use std::path::{Path, PathBuf};

    /// Folders where Android apps read fonts:
    /// 1. Standard external font directory (/storage/emulated/0/Fonts and /sdcard/Fonts)
    ///    used by Phonto, PixelLab, ArtCraft apps, Canva, video editors, etc.
    /// 2. FontCraft subfolder in Fonts (/storage/emulated/0/Fonts/FontCraft).
    /// 3. Download/Fonts (scoped-storage compatible).
    /// 4. Android media folder (/storage/emulated/0/Android/media/app.fontcraft/Fonts).
    /// 5. App private fonts directory as fallback.
    pub fn font_dirs() -> Vec<PathBuf> {
        let mut dirs = vec![
            PathBuf::from("/storage/emulated/0/Fonts/FontCraft"),
            PathBuf::from("/storage/emulated/0/Fonts"),
            PathBuf::from("/sdcard/Fonts/FontCraft"),
            PathBuf::from("/sdcard/Fonts"),
            PathBuf::from("/storage/emulated/0/Download/Fonts"),
            PathBuf::from("/storage/emulated/0/Android/media/app.fontcraft/Fonts"),
        ];
        let h = crate::paths::home();
        if !h.as_os_str().is_empty() {
            dirs.push(h.join("fonts"));
        }
        dirs
    }

    /// Injects (on) or removes a font so other applications on Android can use it.
    pub fn set(path: &Path, on: bool) -> bool {
        let mut placed = false;
        let file_name = path.file_name();
        for dir in font_dirs() {
            if on {
                if super::place_in(&dir, path) {
                    placed = true;
                    if let Some(name) = file_name {
                        notify_file(&dir.join(name), true);
                    }
                }
            } else {
                if super::take_from(&dir, path) {
                    placed = true;
                    if let Some(name) = file_name {
                        notify_file(&dir.join(name), false);
                    }
                }
            }
        }
        // Broadcast custom intent so ArtCraft apps and listening receivers reload fonts
        notify_apps(path, on);
        placed
    }

    /// Notifies Android's MediaScanner that a font file was added or removed.
    fn notify_file(target: &Path, on: bool) {
        let _ = on;
        let uri = format!("file://{}", target.display());
        let _ = crate::paths::command("am")
            .args(["broadcast", "-a", "android.intent.action.MEDIA_SCANNER_SCAN_FILE", "-d", &uri])
            .status();
    }

    /// Broadcasts font activation/deactivation event to ArtCraft suite and other Android apps.
    fn notify_apps(path: &Path, on: bool) {
        let action = if on { "app.fontcraft.FONT_ACTIVATED" } else { "app.fontcraft.FONT_DEACTIVATED" };
        let p = path.to_string_lossy();
        let _ = crate::paths::command("am")
            .args(["broadcast", "-a", action, "--es", "font_path", &p])
            .status();
    }

    /// Removes leftover activated fonts on startup, keeping only the desired ones.
    pub fn sweep(keep: &[PathBuf]) {
        let fc_dirs = [
            PathBuf::from("/storage/emulated/0/Fonts/FontCraft"),
            PathBuf::from("/sdcard/Fonts/FontCraft"),
            PathBuf::from("/storage/emulated/0/Android/media/app.fontcraft/Fonts"),
        ];
        let wanted: Vec<std::ffi::OsString> = keep.iter().filter_map(|p| p.file_name().map(|n| n.to_os_string())).collect();
        for dir in &fc_dirs {
            let Ok(rd) = std::fs::read_dir(dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() && !wanted.iter().any(|w| *w == e.file_name()) {
                    let _ = std::fs::remove_file(&p);
                    notify_file(&p, false);
                }
            }
        }
    }
}

/// Load (`on`) or unload a font file for other apps. Returns whether it worked.
pub fn set(path: &Path, on: bool) -> bool {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        mac::set(path, on)
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android")))]
    {
        // Linux: the file in ~/.local/share/fonts/FontCraft, then fontconfig reads it again. A hard
        // link (a real file there) when the font is on the same disk, so apps that read the font
        // folders themselves and skip links (the ArtCraft apps) see it too; else a symbolic link.
        let dir = crate::paths::home().join(".local/share/fonts/FontCraft");
        let ok = if on { place_in(&dir, path) } else { take_from(&dir, path) };
        // Open apps notice new fonts when fontconfig's cache and the font folders' times change:
        // GNOME's settings daemon watches the font folders and tells GTK apps (and LibreOffice's
        // GTK/KDE front ends) through the Fontconfig/Timestamp setting; others check the folders'
        // times with FcConfigUptoDate. So the cache is rebuilt for the whole user font folder and
        // its time is brought up to date, not only FontCraft's subfolder.
        let parent = crate::paths::home().join(".local/share/fonts");
        let _ = crate::paths::command("fc-cache").arg("-f").arg(&parent).status();
        let _ = crate::paths::command("touch").arg(&parent).arg(&dir).status();
        ok
    }
    #[cfg(target_os = "android")]
    {
        android::set(path, on)
    }
    #[cfg(target_os = "windows")]
    {
        win::set(path, on)
    }
}

/// Puts the font file into a font folder: a hard link (same disk), else a symbolic link (Linux),
/// else a copy (macOS and Windows: macOS and the apps that read the folders skip symbolic links).
/// The name keeps the file's own; another font of that name gets a numbered one.
fn place_in(dir: &Path, path: &Path) -> bool {
    let Some(target) = slot(dir, path) else { return false };
    if target.exists() {
        return true;
    }
    std::fs::create_dir_all(dir).is_ok()
        && (std::fs::hard_link(path, &target).is_ok() || {
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                std::os::unix::fs::symlink(path, &target).is_ok() || std::fs::copy(path, &target).is_ok()
            }
            #[cfg(any(not(unix), target_os = "macos"))]
            {
                std::fs::copy(path, &target).is_ok()
            }
        })
}

/// Takes the font's file out of the font folder again.
#[cfg_attr(any(target_os = "macos", target_os = "ios"), allow(dead_code))]
fn take_from(dir: &Path, path: &Path) -> bool {
    match slot(dir, path) {
        Some(t) if t.exists() || t.symlink_metadata().is_ok() => std::fs::remove_file(&t).is_ok(),
        _ => true,
    }
}

/// Where a font file goes in `dir`: its own name, or "name 2.ext", "name 3.ext"… when a different
/// font already has that name (a file already there for the same font is reused).
#[cfg_attr(target_os = "android", allow(dead_code))]
fn slot(dir: &Path, path: &Path) -> Option<std::path::PathBuf> {
    let name = path.file_name()?.to_string_lossy().to_string();
    let (stem, ext) = match name.rsplit_once('.') {
        Some((a, b)) => (a.to_string(), format!(".{b}")),
        None => (name.clone(), String::new()),
    };
    let size = std::fs::metadata(path).ok()?.len();
    for n in 1..100 {
        let cand = dir.join(if n == 1 { name.clone() } else { format!("{stem} {n}{ext}") });
        let same = std::fs::canonicalize(&cand).ok().is_some_and(|c| std::fs::canonicalize(path).ok().as_ref() == Some(&c))
            || std::fs::metadata(&cand).ok().is_some_and(|m| m.len() == size && std::fs::read(&cand).ok() == std::fs::read(path).ok());
        if same || cand.symlink_metadata().is_err() {
            return Some(cand);
        }
    }
    None
}

#[cfg(target_os = "windows")]
mod win {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    #[link(name = "gdi32")]
    extern "system" {
        fn AddFontResourceExW(name: *const u16, fl: u32, res: *mut std::ffi::c_void) -> i32;
        fn RemoveFontResourceExW(name: *const u16, fl: u32, res: *mut std::ffi::c_void) -> i32;
    }
    #[link(name = "user32")]
    extern "system" {
        fn SendNotifyMessageW(hwnd: isize, msg: u32, w: usize, l: isize) -> i32;
    }
    const HWND_BROADCAST: isize = 0xffff;
    const WM_FONTCHANGE: u32 = 0x001d;

    /// Windows: the font is added for this session (every app), then apps are told fonts changed.
    /// Its file also goes into the user's font folder (%LOCALAPPDATA%\\Microsoft\\Windows\\Fonts\\
    /// FontCraft), without registering it there, for apps that read that folder themselves (the
    /// ArtCraft apps); Windows itself only loads the fonts registered for it.
    pub fn set(path: &Path, on: bool) -> bool {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let dir = Path::new(&local).join("Microsoft").join("Windows").join("Fonts").join("FontCraft");
            if on {
                super::place_in(&dir, path);
            } else {
                super::take_from(&dir, path);
            }
        }
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let ok = unsafe {
            if on {
                AddFontResourceExW(wide.as_ptr(), 0, std::ptr::null_mut()) > 0
            } else {
                RemoveFontResourceExW(wide.as_ptr(), 0, std::ptr::null_mut()) != 0
            }
        };
        unsafe {
            SendNotifyMessageW(HWND_BROADCAST, WM_FONTCHANGE, 0, 0);
        }
        ok
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    /// A font turned on is registered and linked into ~/Library/Fonts/FontCraft, and its link
    /// comes out again when turned off. Uses a downloaded Google font when there is one.
    #[test]
    fn on_and_off_through_the_user_font_folder() {
        let gf = crate::gfonts::dir().join("Open Sans");
        let Some(src) = std::fs::read_dir(&gf).ok().and_then(|rd| rd.flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x == "ttf"))) else { return };
        let tmp = std::env::temp_dir().join(format!("fontcraft-act-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let font = tmp.join("FontCraftTest-Probe.ttf");
        std::fs::copy(&src, &font).unwrap();
        assert!(super::set(&font, true), "turned on");
        let placed = super::mac::folder().join("FontCraftTest-Probe.ttf");
        assert!(placed.exists(), "placed in the user font folder");
        assert!(super::set(&font, false), "turned off");
        assert!(!placed.exists(), "taken out again");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
