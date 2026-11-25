//! Values taken from RightFont itself (addresses in its main program; details in MECHANICS.md).

/// Preview size in the font list when the person never changed it.
/// -[RFPreferencesHandler fontPreviewListViewFontSize] at 0x1000aa540 returns 16 when unset.
pub const LIST_PREVIEW_SIZE: f32 = 16.0;

/// Preview size of the sample (single font) view when never changed.
/// -[RFPreferencesHandler fontPreviewSampleViewFontSize] at 0x1000aa494 returns 24 when unset.
pub const SAMPLE_PREVIEW_SIZE: f32 = 24.0;

/// Glyph size on the Glyphs page. RightFont's default is 24 (-[RFPreferencesHandler glyphFontSize]
/// at 0x1000aa7d4 returns 24 when unset); the person asked for 72 points.
pub const GLYPH_SIZE: f32 = 72.0;

/// The Glyphs page's own size buttons: ±2 points, from 56 to 128.
/// -[RFFontGlyphViewController increaseFontSize:] at 0x1000b7388 adds 2 while the size is ≤ 126
/// (constant at 0x100282518); decreaseFontSize: at 0x1000b7300 takes 2 while it is ≥ 58.
pub const GLYPH_STEP: f32 = 2.0;
pub const GLYPH_MIN: f32 = 56.0;
pub const GLYPH_MAX: f32 = 128.0;

/// The details panel's Preview tab always shows its samples at 60 points (the person's request).
pub const PANEL_PREVIEW_SIZE: f32 = 60.0;

/// Bigger / smaller font size step (menu or keys): the slider value plus or minus 4,
/// never past the slider's limit. -[RFMainWindowController increaseFontSizeNotified:] at 0x10003b4c4.
pub const SIZE_STEP: f32 = 4.0;

/// A font row in the list (-[RFFontItemView updateTextRenderLayerFrame] at 0x100068570): the sample
/// is drawn from 23 points below the row's top, 12 points in from each side, and ends 5 points above
/// its bottom (frame = 12, 23, width − 24, height − 28), so a row is the sample's height + 28.
pub const ROW_SAMPLE_TOP: f32 = 23.0;
pub const ROW_SAMPLE_BOTTOM: f32 = 5.0;
pub const ROW_SIDE: f32 = 12.0;
/// The row's icons sit on a line 15 points below its top (activation icon 14 × 14 at y 8,
/// -[RFFontItemView updateLayer] at 0x100068b4c).
pub const ROW_ICON_Y: f32 = 15.0;
/// The star button: 16 × 16 at (width − 24, 8), so its centre is 16 points from the right edge
/// (0x100068b4c).
pub const STAR_FROM_RIGHT: f32 = 16.0;

/// The size slider's limits aren't in the code (they come from the window layout file); estimated.
pub const SIZE_MIN: f32 = 10.0;
pub const SIZE_MAX: f32 = 160.0;

/// The first macOS major version FontCraft hasn't been tested on: RightFont counts a system as
/// compatible while `NSProcessInfo.operatingSystemVersion.majorVersion < 28` (+[RFGlobals
/// isCompatibleSystemVersion] at 0x1000e97f0, `cmp x8, #0x1c; cset w0, lt`), and otherwise shows
/// "There may be compatibility issues with the %@ of macOS, it is recommended to upgrade to the
/// latest version of RightFont." (string 0x100285eb3, shown from 0x100019e58).
pub const UNTESTED_MACOS_MAJOR: u32 = 28;

/// Going back to the menu bar from full screen: the longest wait for macOS to say the window has
/// left full screen (windowDidExitFullScreen; its animation takes about half a second) before the
/// window is hidden anyway. Estimated; RightFont has no full-screen window (its panel only hides).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const LEAVE_FULLSCREEN_MAX_S: f64 = 3.0;

/// An app that reads the installed fonts only when it opens (TraceDraw: fontdb scans the font
/// folders once, "system fonts read in 144 ms" in its log, no folder watcher, no CoreText
/// notice) shows a font FontCraft turned on while it was open only after it is reopened: the
/// font stays this long after the app closes, so reopening it lists the font; then it goes, as
/// it does when FontCraft quits. Estimated.
pub const REOPEN_GRACE_S: f64 = 60.0;

/// The longest wait for DesignCraft to answer a command script over its control channel
/// (it answered at once in the tests; a stuck app mustn't hold FontCraft's worker). Estimated.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const CHANNEL_WAIT_S: f64 = 5.0;

/// PressCraft's font field, filled through Accessibility (presscraft.rs): the pause after each step
/// (the field taking the keyboard, the family typed in, the style menu opened) for PressCraft to
/// draw a frame; the longest wait for the field to show the family after Return; how many times its
/// interface is asked for (AccessKit sends PressCraft's widgets to macOS only after the first
/// question, a frame later), every 250 ms; and the most elements looked at. Estimated.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const PRESSCRAFT_STEP_MS: u64 = 220;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const PRESSCRAFT_PICK_WAIT_MS: u64 = 1500;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const PRESSCRAFT_TREE_TRIES: u32 = 8;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const PRESSCRAFT_TREE_NODES: usize = 6000;

/// FontCraft opened again (Finder, Dock, Launchpad, Spotlight) while it works in the menu bar
/// shows its window, as RightFont does (-[RFAppDelegate applicationShouldHandleReopen:
/// hasVisibleWindows:], 0x1000db7fc) — but not within this long of starting or of a trip to or
/// from the menu bar, when macOS activates the app on its own. Estimated (RightFont gets the reopen
/// event itself; FontCraft's window library doesn't pass it on, so the activation is watched).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const REOPEN_SETTLE_MS: u64 = 2500;

/// FontCraft opened again from the Finder while in the menu bar: how long macOS is left to finish
/// bringing it forward before its window comes up and goes full screen (a full-screen move started
/// at once was given up by macOS in the test on this Mac). Estimated.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const REOPEN_DELAY_S: f64 = 0.35;

/// Out of the menu bar, full screen: first look this long after the window comes up, then every
/// twice this long, FULLSCREEN_LOOKS times; asked of macOS at most FULLSCREEN_ASKS times; a move
/// macOS started and never finished counts as given up after FULLSCREEN_MOVE_STALE_MS (the move
/// itself took 0.6 to 0.8 s on this Mac: willEnter 3846 ms, didEnter 4646 ms; once macOS sent
/// willEnter at 11815 ms and never didEnter, and the second ask at 14044 ms finished at 14655 ms).
/// Estimated.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const FULLSCREEN_LOOK_S: f64 = 0.15;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const FULLSCREEN_LOOKS: u32 = 12;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const FULLSCREEN_ASKS: u32 = 2;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const FULLSCREEN_MOVE_STALE_MS: u64 = 1500;

/// The running macOS version ("26.7.3"), or None elsewhere.
pub fn macos_version() -> Option<String> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let out = crate::paths::command("sw_vers").arg("-productVersion").output().ok()?;
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!v.is_empty()).then_some(v)
}

/// Whether FontCraft has been tested on a macOS version (RightFont's rule above).
pub fn is_compatible_macos(version: &str) -> bool {
    version.split('.').next().and_then(|m| m.parse::<u32>().ok()).is_none_or(|m| m < UNTESTED_MACOS_MAJOR)
}

#[cfg(test)]
mod tests {
    #[test]
    fn compatible_systems() {
        assert!(super::is_compatible_macos("26.7.3"));
        assert!(super::is_compatible_macos("27.0"));
        assert!(!super::is_compatible_macos("28.1"));
        assert!(super::is_compatible_macos(""));
    }
}
