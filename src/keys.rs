//! The keys that type a character with the keyboard layout in use (macOS): every key with no
//! modifier, ⇧, ⌥ and ⌥⇧ is asked of the layout (UCKeyTranslate on the layout's 'uchr' data), and
//! dead keys (⌥E then E for "é") are followed one step. Shown on the glyph preview and in
//! Character Info as "⌥⇧2" or "⌥E, E". Built once per layout, on the main thread.

use std::collections::HashMap;

/// The keys for `ch`, as written on a Mac ("⌥⇧2", "⌥E, E"), or None when no key types it.
pub fn shortcut_for(ch: char) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        mac::shortcut_for(ch)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = ch;
        None
    }
}

/// Modifiers in the order Mac menus write them.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn mods_label(m: u32) -> String {
    let mut s = String::new();
    if m & 0x08 != 0 {
        s.push('⌥');
    }
    if m & 0x02 != 0 {
        s.push('⇧');
    }
    s
}

/// What is printed on the key.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn key_label(code: u16, base: Option<char>) -> String {
    match code {
        49 => "Espacio".into(),
        36 => "↩".into(),
        48 => "⇥".into(),
        _ => base.map(|c| c.to_uppercase().collect()).unwrap_or_else(|| format!("#{code}")),
    }
}

#[allow(dead_code)]
type Map = HashMap<char, String>;

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use std::ffi::c_void;
    use std::sync::Mutex;

    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn TISCopyCurrentKeyboardLayoutInputSource() -> *const c_void;
        fn TISGetInputSourceProperty(src: *const c_void, key: *const c_void) -> *const c_void;
        static kTISPropertyUnicodeKeyLayoutData: *const c_void;
        static kTISPropertyInputSourceID: *const c_void;
        fn UCKeyTranslate(layout: *const c_void, code: u16, action: u16, mods: u32, kbd: u32, opts: u32, dead: *mut u32, max: usize, len: *mut usize, out: *mut u16) -> i32;
        fn LMGetKbdType() -> u8;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFDataGetBytePtr(d: *const c_void) -> *const u8;
        fn CFRelease(p: *const c_void);
    }

    const MODS: [u32; 4] = [0, 0x02, 0x08, 0x0a];

    struct Cache {
        id: String,
        map: Map,
    }

    static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

    fn translate(layout: *const c_void, kbd: u32, code: u16, mods: u32, dead: &mut u32) -> Option<String> {
        let mut buf = [0u16; 8];
        let mut len = 0usize;
        let r = unsafe { UCKeyTranslate(layout, code, 0, mods, kbd, 0, dead, buf.len(), &mut len, buf.as_mut_ptr()) };
        if r != 0 || len == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..len]))
    }

    fn layout_id(src: *const c_void) -> String {
        let id = unsafe { TISGetInputSourceProperty(src, kTISPropertyInputSourceID) };
        if id.is_null() {
            return String::new();
        }
        let s: core_foundation::string::CFString = unsafe { core_foundation::base::TCFType::wrap_under_get_rule(id as core_foundation::string::CFStringRef) };
        s.to_string()
    }

    fn build(layout: *const c_void) -> Map {
        let kbd = unsafe { LMGetKbdType() } as u32;
        let mut map = Map::new();
        let base = |code: u16| -> Option<char> {
            let mut d = 0;
            translate(layout, kbd, code, 0, &mut d).and_then(|s| s.chars().next()).filter(|c| !c.is_control())
        };
        let codes: Vec<u16> = (0u16..=50).collect();
        // Plain keys first, so the simplest way to type a character wins.
        for &m in &MODS {
            for &code in &codes {
                let mut dead = 0u32;
                if let Some(s) = translate(layout, kbd, code, m, &mut dead) {
                    let mut it = s.chars();
                    if let (Some(c), None) = (it.next(), it.next()) {
                        if !c.is_control() {
                            map.entry(c).or_insert_with(|| format!("{}{}", mods_label(m), key_label(code, base(code))));
                        }
                    }
                }
            }
        }
        // Dead keys: the accent key, then a key that completes it.
        for &m in &MODS {
            for &code in &codes {
                let mut dead = 0u32;
                if translate(layout, kbd, code, m, &mut dead).is_some() || dead == 0 {
                    continue;
                }
                let first = format!("{}{}", mods_label(m), key_label(code, base(code)));
                for &m2 in &MODS[..2] {
                    for &k2 in &codes {
                        let mut d2 = dead;
                        if let Some(s) = translate(layout, kbd, k2, m2, &mut d2) {
                            let mut it = s.chars();
                            if let (Some(c), None) = (it.next(), it.next()) {
                                if !c.is_control() {
                                    map.entry(c).or_insert_with(|| format!("{first}, {}{}", mods_label(m2), key_label(k2, base(k2))));
                                }
                            }
                        }
                    }
                }
            }
        }
        map
    }

    pub fn shortcut_for(ch: char) -> Option<String> {
        let run = || unsafe {
            let src = TISCopyCurrentKeyboardLayoutInputSource();
            if src.is_null() {
                return None;
            }
            let id = layout_id(src);
            let mut guard = CACHE.lock().ok()?;
            if guard.as_ref().map(|c| c.id != id).unwrap_or(true) {
                let data = TISGetInputSourceProperty(src, kTISPropertyUnicodeKeyLayoutData);
                let map = if data.is_null() { Map::new() } else { build(CFDataGetBytePtr(data) as *const c_void) };
                *guard = Some(Cache { id, map });
            }
            CFRelease(src);
            guard.as_ref().and_then(|c| c.map.get(&ch).cloned())
        };
        std::panic::catch_unwind(run).ok().flatten()
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn letters_have_keys() {
        let a = super::shortcut_for('a');
        assert!(a.is_some(), "{a:?}");
        println!("€ → {:?}, é → {:?}, ñ → {:?}", super::shortcut_for('€'), super::shortcut_for('é'), super::shortcut_for('ñ'));
    }
}
