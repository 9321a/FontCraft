# FontCraft — values and rules taken from RightFont

Source: `/Applications/RightFont.app/Contents/MacOS/RightFont` (Mach-O, arm64, 16 476 functions),
read with Craft Anything's built-in decompiler (Ghidra was unavailable, so register-level pseudo-C).
Addresses are function entry points in that program. "Estimated" marks what the code didn't show.

## Font activation (the asterisk)

`-[RFFontActivator activateFontByName:andURL:andEnabled:]` — **0x10004b974**

- Turning on: `CTFontManagerRegisterFontsForURL(url, 2, &error)` — scope **2 = persistent**: the font
  stays on for every app, also after logging out and restarting.
- Before registering, it asks `CTFontManagerGetScopeForURL(url)`; if the font is already registered
  with another scope, it is unregistered from that scope first.
- Turning off: `CTFontManagerUnregisterFontsForURL(url, <current scope>, &error)`.
- Then `CTFontManagerEnableFontDescriptors(descriptors, enabled)`.
- For fonts registered by descriptor it uses `CTFontManagerRegisterFontDescriptors(…, 2, true, …)`.

→ `src/activate.rs` now uses scope 2 with the same "unregister other scope first" rule
(before: session scope 3, fonts went off at logout).

## Preview sizes

| Setting (defaults key) | Default when unset | Function |
|---|---|---|
| `FontPreviewListViewFontSize` (font list) | **16** | `-[RFPreferencesHandler fontPreviewListViewFontSize]` **0x1000aa540** |
| sample view (one font) | **24** | `-[RFPreferencesHandler fontPreviewSampleViewFontSize]` **0x1000aa494** |
| `FontGlyphsFontSize` (glyphs) | **24** | `-[RFPreferencesHandler glyphFontSize]` **0x1000aa7d4** |

- Bigger/smaller size: `new = slider.integerValue + 4`, capped at the slider's maximum
  (`-[RFMainWindowController increaseFontSizeNotified:]` **0x10003b4c4**; decrease mirrors it at
  **0x10003b5d0**). → step **4**, keys ⌘+ / ⌘−.
- The slider's min/max are set in the window layout file, not in code → kept at 10–160 (estimated).

→ `src/mechanics.rs`: `LIST_PREVIEW_SIZE = 16` (new default), `GLYPH_SIZE = 24` (glyph grid),
`SAMPLE_PREVIEW_SIZE = 24` (feature examples), `SIZE_STEP = 4`.

## Libraries and font lists

Storage: one SQLite database (`core-v10.db` in `~/Library/Application Support/RightFont`) with tables
`library`, `collection` (= font list), `font`, `fontCollection`, `tag`. Collection columns:
`id, library_id, name, type, location, rule, icon, parent, comments, created, created_by, modified, modified_by`.

- **List order in the sidebar** — query string at **0x100296ae0**:
  `WITH RECURSIVE collections AS (SELECT * FROM collection WHERE parent IS NULL AND library_id = ?
  UNION ALL SELECT t.* FROM collection t JOIN collections ON t.parent = collections.id)
  SELECT * FROM collections ORDER BY created;` → lists nest by `parent` and are ordered by **creation time**.
- **A list includes its sub-lists' fonts** — query at **0x10028a4b4** (recursive over `parent`, then the
  fonts of all those list ids).
- **Lists shown for one font** are sorted by name, case-insensitive — query at **0x10028a259**
  (`ORDER BY c.name COLLATE NOCASE ASC`).
- **Adding a folder** — `-[RFFontService loadCollectionsAndFontsFromURLs:andParentCollection:toLiveLibrary:]`
  **0x100080b44**: for each item, `stat()`; a directory (`st_mode & 0xF000 == 0x4000`) becomes a new
  collection named after **its own folder name**, with `parent` = the folder above, `created` =
  `CFAbsoluteTimeGetCurrent()`, `created_by` = `NSFullUserName()`, and its contents are read the same
  way (nested); a font file is added to the collection of the folder it is in.
- **Exported RightFont libraries (`.rightfontlibrary`)**: no longer supported in FontCraft (the
  person's request); FontCraft has its own library format instead (below).

→ `src/libscan.rs`: folders become **nested** lists named after their own folder; a list includes its
sub-lists' fonts. Fonts inside a library are listed by name. Folder reading order for new
collections: RightFont reads them in the order the file system returns them; FontCraft reads them in
name order (estimated).

## FontCraft libraries and folders (FontCraft's own design, person's request)

- **Sidebar**: "LIBRARIES" lists FontCraft libraries (.fcl, books icon) with their **font families by full
  name** underneath ("Aa" icon); "FOLDERS" lists folders added from the computer (folder icon) with their
  subfolders. Folders stay where they are and their fonts are switched on from there.
- **New Library** and **Add Folder** windows are the same (folder, fonts found, name from the folder or
  typed, its subfolders); only the title and button differ. A library copies the folder's fonts into
  `~/FontCraft/<name>.fcl`, filed in A–Z folders.
- **.fcl library** (`src/fcl.rs`): `~/FontCraft/<name>.fcl`, a container folder (like a macOS
  package) with `library.json` (`format: "fontcraft-library"`, `version: 1`, `name`, `created` in Unix
  seconds, `layout: "first-letter-folders"`). `~/FontCraft` is made when the first library is created.
  Fonts are copied into a folder named after the first letter of their family name (typographic family,
  name ID 16, else ID 1): `A`…`Z` (and letters such as `Ñ`), `0-9` for digits, `#` for anything else.
  Each letter folder is a list; lists and fonts are in name order. A file already there is never copied
  over.
- **Import Fonts** (Settings › Library › Import Fonts): "In" + a pop-up menu with **Libraries ▸** and
  **Folders ▸** submenus + **Add…** (the system's Open panel, fonts and folders together) copies into the
  chosen one; "Create" + pop-up (**Library** / **Folder**) + **Add** opens the sidebar's windows.
- **.fcl is FontCraft's document type**: `packaging/macos/Info.plist` declares `app.fontcraft.library`
  (a package, extension `fcl`, FontCraft is its owner, icon `fontcraft-library.icns`);
  `packaging/linux/fontcraft-library.xml` (MIME `application/x-fontcraft-library`) and
  `packaging/windows/fontcraft-library.reg` do the same elsewhere. Opening a .fcl adds it to Libraries.
- **Icons** (`src/appicon.rs`, `--make-icons`): app icon and .fcl document icon drawn at any size up to
  512 × 512 px, light and dark versions (`assets/icons`: .icns, .ico, PNGs). The window uses the one
  matching the system's look. (macOS app bundles can't switch their icon with dark mode unless built with
  Apple's Icon Composer tools, which aren't on this computer; the dark PNGs are ready for that.)
- **Sharing** (Settings › General › Share settings with): "All users" or "User" + one account (people
  with a home folder under /Users, UID ≥ 501); **Apply** takes effect at once: `~/FontCraft` gets an
  access-list entry for that group/user (`list, add_file, search, add_subdirectory, delete_child, read,
  write, …, file_inherit, directory_inherit`), replacing earlier sharing, and the home folder gets only
  `allow search` for them (they can pass through, not look around). Other users' FontCraft then shows
  the shared libraries under My Libraries as "<name> (<user>)".
- **Permissions**: when FontCraft needs a system permission it opens the system's own settings:
  Accessibility (with the system's request prompt, `AXIsProcessTrustedWithOptions`), Automation, or
  Full Disk Access (if sharing is refused); Windows opens Privacy settings.

## Font rows and importing (read from RightFont)

- **Row layout** — `-[RFFontItemView updateTextRenderLayerFrame]` **0x100068570**: the sample's frame is
  `(12, 23, width − 24, height − 28)` → it starts **23 pt** below the row's top, **12 pt** in from each
  side and ends **5 pt** above the bottom; a row is the sample height + 28.
- **Row icons** — `-[RFFontItemView updateLayer]` **0x100068b4c**: activation icon 14 × 14 at (9, 8),
  font-type mark 13 × 13 at (25, 9), name from x 42 (16 pt high, y 8), star button 16 × 16 at
  (width − 24, 8), action button 18 × 18 at (width − 46, 7). `-[RFFontItemView configLayers]`
  **0x1000681a0** creates the font-type layer (16 × 16, corner radius 2) and the style-count layer
  (60 × 18, corner radius 2, 11 pt text). So RightFont does show a format mark next to the activation icon.
- **Row height** — `-[RFFontListViewController updateRowHeight]` **0x100104604**: `ceil(text height) + 8`;
  `updateRowHeightPreservingVisiblePosition` **0x100104c50** keeps the same fraction of the list in view
  when the height changes.
- **Sidebar rows** — `-[RFNavigationViewController outlineView:heightOfRowByItem:]` **0x100130bd8**: 32 pt
  or 26 pt depending on the kind of item.
- **Variable fonts** — `-[RFFontFileHandler fontHasVariations:]` **0x1000fed50**: variable when
  `CTFontCopyVariation` or `CTFontCopyVariationAxes` returns something. FontCraft: the font has an `fvar`
  table (same fonts), shown as "VAR".
- **Format names** — strings "TrueType" (0x10028a614) and "PostScript" (0x10028a61d); the font table keeps
  `format`, `fileType`, `hasVariations`, `hasColorGlyphs` (query string at 0x100287e86).
- **Duplicates on import** — `-[RFFontFileHandler existingFontMatchingFont:]` **0x1000ff100** looks the new
  font up among the existing ones by name (`existingFontsByName`); a match isn't imported again.

→ `src/mechanics.rs` + `src/main.rs`: rows use RightFont's 23 pt top, 5 pt bottom and 12 pt sides (before:
30 / 10 / 20 + 4, estimated), icons on the line 15 pt down, star centred 16 pt from the right edge (before
22). `src/fcl.rs`: importing skips a font whose PostScript name the library already has, even under another
file name (before: same file name only). Icon size stays 18 pt for all four icons (the person's request;
RightFont's are 13–16 pt).

## Font list rows (person's request)

- Each row is as tall as its sample needs: `row = top + size × extent + gap`, where `extent` (in ems)
  runs from the highest letter of the sample (or the font's ascender) to the lowest (or the descender),
  plus the line gap; the picture itself also grows to hold every letter's ink. So nothing is cut off at
  any size, and rows change height as the size changes.
- Above the sample: asterisk (or padlock), then the **format** icon (a page with "OTF", "TTF", "TTC",
  "OTC" or "VAR"), then the family name and style count; the favourite **star** on the right. All four
  icons share one 18-point design: round-ended 1.6–2.6 pt strokes, grey `#9A9AA2` when off; on: lime
  asterisk, filled amber star `#F5A623`.

## Auto Activation apps (person's request)

- FontCraft looks for the supported apps every time it starts and whenever the Auto Activation settings
  open, so apps installed or removed since appear or disappear. macOS: bundle ids in the Applications
  folders; Linux: desktop entries (system, user, Flatpak, Snap) by file or program name; Windows: program
  files in Program Files and the user's programs. Icons 24 pt (12–24 allowed), read at 64 px, never
  stretched. The list sits in a scrolling box, the options beside it.
- What FontCraft can do with each app (checked, not assumed): **Script** (RightFont's AppleScripts:
  Photoshop, Illustrator, InDesign, InCopy, After Effects, QuarkXPress, OmniGraffle, Word, Excel,
  PowerPoint, Pages, Keynote, TextEdit, Pixelmator Pro), **Paste** (rich text with its font is pasted back:
  the rest of RightFont's list, LibreOffice, ONLYOFFICE, CorelDRAW, Lunacy, Linearity Curve, and the
  Windows versions of Office/Adobe/Affinity/Figma), **fonts only** (the app takes fonts from the system but
  not from pasted text and has no script: VectorCraft, DesignCraft, PhotoCraft, PdfCraft, Inkscape,
  Scribus, Krita, GIMP, Motion). Left out on purpose: TextureCraft and SolidCraft (3D, no type tools),
  Blender and FontLab (they load font files themselves, so switching fonts on does nothing for them).
- Outside macOS there are no AppleScript or keystrokes to send, so every app is "fonts only" there; fonts
  are switched on with `AddFontResourceExW` + `WM_FONTCHANGE` (Windows) or a link in
  `~/.local/share/fonts/FontCraft` + `fc-cache` (Linux); inserting a character copies it for pasting.

## Searching fonts and glyphs (read from RightFont)

- **Font search** — query part at **0x100285208**: `(font.name LIKE '%x%' OR font.fullName LIKE '%x%' OR
  font.displayName LIKE '%x%' OR font.familyGroup LIKE '%x%' OR font.style LIKE '%x%' OR font.manufacturer
  LIKE '%x%' OR font.designer LIKE '%x%' OR font.copyright LIKE '%x%' OR font.license LIKE '%x%')`
  (a shorter one at 0x100285321 uses only the four names). Lists, libraries and tags are matched with
  `LIKE '%x%'`, `'x%'` and `'%x'` (0x10029293b…0x100292d86).
- **Glyph filter** — `-[RFFontGlyphViewController filterGlyphs]` **0x1000b7050** builds one of three
  filters: `SELF.unicodeCategory == %@ AND SELF.character == %@` (0x10029142d), `SELF.unicodeCategory == %@`
  (0x100291461) or `SELF.character == %@` (0x10029147c); with neither, every glyph shows.
  `searchFieldDidStartSearching:` **0x1000b701c** starts it as the person types.

→ `src/main.rs`: FontCraft's search looks, case-insensitively, through each family's name, its styles and
full names, and the default style's typographic family, manufacturer, designer, copyright and license
(before: the family name only). `src/panel.rs`: the Glyphs page has a character box (a character, `U+0041`
or `0041`) and a kind menu — all, uppercase, lowercase, numbers, punctuation, symbols, unmapped — that can
be used together, like RightFont's filters (FontCraft's kinds come from the characters themselves; RightFont's
category names aren't in its program).

## Font classes (read from RightFont)

- `-[RFFont classificationString]` **0x10009ee74**: a font's classification is a number 1–9 looked up
  in the table at **0x10030f048** (strings from 0x100285ddf): **1 Sans Serif, 2 Serif, 3 Slab Serif,
  4 Script, 5 Blackletter, 6 (empty), 7 Display, 8 Symbols, 9 Monospaced**; otherwise the stored text.
- `-[RFPreferencesHandler loadFontClassifications]` **0x1000abae4** reads the classifications from a file
  in the app's resources and keeps them; `-[RFFontService classifyFonts:withClassification:]` 0x10007dca0
  lets a person set a font's class by hand.

→ `src/fonts.rs` + `src/main.rs`: FontCraft has the same classes and shows the smart lists in RightFont's
order: Sans Serif, Serif, **Slab Serif**, Script, **Blackletter**, **Display**, Symbols, Monospaced (before:
Monospaced, Serif, Sans Serif, Script, Symbols). How FontCraft decides (RightFont's own rules for its
classes aren't in its program; they come with its data): blackletter by name (Fraktur, Textura, Old
English…); script as before; display = PANOSE family 4 "Latin Decorative", IBM class 9 "Ornamentals", or
names such as Display, Stencil, Poster, Inline, Outline; slab = IBM class 5 "Slab Serifs" or names such as
Slab, Rockwell, Clarendon, Egyptian, American Typewriter (IBM class 4 "Clarendon" also covers fonts like
Georgia, so it counts as serif).

## Where libraries live, folder watching, Google Fonts updates (read from RightFont)

- `-[RFPreferencesHandler libraryDefaultLocation]` **0x1000ab780**: `NSHomeDirectory()` + `"%@/RightFont"`
  (string at 0x100290eda) → RightFont keeps libraries in a folder named after the app in the home folder;
  its data is in `"%@/Library/Application Support/RightFont"` (0x100294aef). FontCraft: `~/FontCraft` and
  `~/Library/Application Support/FontCraft` (same pattern; Windows/Linux use their own data folders).
- `-[RFPreferencesHandler importFolders]` **0x1000a9140**: off by default.
- **Folder watching** — `-[RFFileWatcher start]` (block **0x10000c420**):
  `FSEventStreamCreate(NULL, callback, ctx, paths, kFSEventStreamEventIdSinceNow, 0.25, 0x1F)` on its own
  dispatch queue; flags 0x1F = use CF types | no defer | watch root | ignore self | file events.
- **Google Fonts updates** — `-[RFGoogleFontImporter importGoogleFonts]` **0x1001216e4** imports each family;
  `deleteOldVersionGoogleFontsByFamilies:` **0x100121574** removes the older version of those families.
  FontCraft already does this: a family is downloaded to a temporary folder and replaces the old folder whole.

→ `src/watch.rs` + `src/main.rs`: FontCraft watches every library and folder with the same FSEvents call
(0.25 s, flags 0x1F) and reads them again 1 s after the last change, so fonts added, removed or renamed
in Finder or another app appear by themselves (before: only after a restart). Windows/Linux: the folders'
modification times are checked every 3 s.

## App versions in Auto Activation (read from RightFont)

- `-[RFFontActivationPreferencesViewController versionStringForApp:]` **0x100005f14** reads each app's
  `CFBundleShortVersionString` (key string at 0x100296281), falls back to an empty string when it has none,
  and joins the parts with `"%@ %@"` (0x10028b54d) for the app list.
- RightFont itself: version 10.2.1 (build 4668), needs macOS 11.0 or later, category
  `public.app-category.graphics-design`, built for x86_64 and arm64 (its Info.plist).

→ `src/autoact.rs`: the Auto Activation list shows each app's version next to its name (macOS; other
systems don't keep it in a standard place). `packaging/macos/Info.plist`: FontCraft declares the same
category (graphics & design) and the same minimum system (macOS 11).

## Open families and folded lists (read from RightFont)

- `-[RFFontListViewController expandedKeyForItem:]` **0x1001081a8**: a family row's key is
  `"%@-%@"` of two of its names (its library and family); the open ones are kept in `expandedFamilyIDs`
  and saved under the setting **`ExpandedFontFamilies`** (string at 0x10029775b).
- `-[RFFontListViewController restoreExpandedState]` **0x1001082a0**: after the list reloads it goes
  through every row and opens the ones whose key is in that set.
- `-[RFNavigationViewController outlineViewItemDidExpand:]` **0x100130e08** (and `…DidCollapse:`
  0x100130f58) record each sidebar item's open/folded state as it changes; `outlineView:isItemExpandable:`
  **0x10012e888**: an item can open only when it has children.

→ `src/main.rs`, `src/settings.rs`, `src/libs.rs`, `src/libui.rs`: open families (by name) and folded
libraries (by path) are saved in the settings (`expanded=`, `collapsed=`) and restored at start and after
every rescan (before: forgotten each time the fonts were read again or FontCraft restarted).
Also: a list's count in the sidebar looks only at that list's families (it went through every family for
each of the 1,004 lists of a big folder on every frame, which made the window take ~1 minute to open).

## Glyphs page (read from RightFont)

- **Size buttons** — `-[RFFontGlyphViewController increaseFontSize:]` **0x1000b7388**: `size += 2` while
  `size ≤ 126` (constant at **0x100282518**); `decreaseFontSize:` **0x1000b7300**: `size −= 2` while
  `size ≥ 58` → range **56–128 pt, step 2**. `changeFontSize:` **0x1000b74b8** halves the size for one
  case it checks (the inner preview), not used here.
- **Right-click menu** — `collectionView:menuAtIndexPath:` **0x1000b650c** builds an NSMenu with a single
  item whose action is `copy:` (selector at 0x100378298). FontCraft keeps the longer menu the person asked
  for; its "Copy Character" is the same action.
- **Copy** — `-[RFFontGlyphViewController copy:]` **0x1000b7208**: declares one pasteboard type and puts the
  glyph's character on it as a string.
- **Double-click** — `collectionView:doubleClickedAtIndexPath:` **0x1000b6504** sends the same `copy:`.
- **Drag** — `collectionView:writeItemsAtIndexPaths:toPasteboard:` **0x1000b68c8**:
  `NSSearchPathForDirectoriesInDomains(13 = Caches, user)` — the dragged glyph is written as a file in the
  Caches folder and handed over with the drag.

→ `src/panel.rs` / `src/mechanics.rs`: the Glyphs page has −/+ buttons (±2 pt, 56–128 pt, starting at the
person's 72 pt). Double-click puts the glyph into the target app, or copies it when there is none
(RightFont's double-click = copy). `src/main.rs` + `src/dragout.rs`: a dragged glyph also carries a PNG
file written to `~/Library/Caches/FontCraft` and the PNG data, besides the character as plain and rich text.

## Samples, glyphs and printing (person's request)

- Letters a font doesn't have are drawn in the system's own font (San Francisco / Segoe UI / the desktop
  font, then wide-coverage system fonts) instead of empty boxes.
- Wrapped lines are stacked by their real ink: `next baseline = previous + max(line height, previous
  descent ink + this ascent ink + 6 % of the size)`, so lines never overlap. Details › Preview is fixed
  at 60 pt; Details › Glyphs at 72 pt, each glyph scaled down only if it wouldn't fit its cell.
- Glyph right-click menu: Character Info (Unicode, decimal, HTML, UTF-8, UTF-16, block, glyph name, ID,
  advance, font) — Copy character / HTML (`&#N;`) / decimal / hex (`U+XXXX`) — Mark/Unmark fonts that have
  it (red dot, 13 pt, left of the star) — Find characters with a similar shape: every family's default face
  (on or off), up to 3,000 characters each, compared as 16 × 16 ink grids of the glyph's box plus its
  proportions (`distance = mean |difference| + 0.25 × |ln(aspect ratio)|`, kept under 0.16, best 60), shown
  at 96 pt; clicking one inserts it into the target app (or copies it).
- Dragging a glyph past the window's edge starts a system drag (macOS) carrying the character as plain and
  rich text in its font, so it can be dropped into any app; its font is switched on.
- **Print** (printer button next to Settings, ⌘P): all fonts / the current list / favourites, every style
  or not; layouts: one sample line per font (with names and details), a specimen per family, or names only
  in 1–4 columns; size and sample text; the default printer's paper (macOS `NSPrintInfo`). Pages are drawn
  at 2 px per point, saved as a PDF and handed to the system's print dialog (macOS: PDFKit print panel).

## System look (person's request)

- The interface uses the system's own font (San Francisco on macOS, Segoe UI on Windows, the desktop
  font on Linux) and the system's control style: push buttons, check boxes, radio buttons, pop-up menus
  (on macOS with the accent-coloured double arrow) and text fields with an accent focus ring. Sizes:
  macOS 13 pt text / 22 pt controls / 14 pt boxes; Windows 30 px controls / 18 px boxes; Linux 30 px /
  16 px.
- Settings, Import Fonts, New Library / Add Folder and Google Fonts open as real system windows (system
  frame, title and close button); each fits its content and resizes when the content changes (switching
  Settings tabs).
- The toolbar lays out its right-hand group (settings, theme, target app) first, so the settings button
  is always fully visible.

## Google Fonts

- RightFont doesn't use Google's servers directly: it loads its catalogue from its own server,
  `https://api.rightfontapp.com/v3/font/?method=get&token=googlefontsVF` (string at 0x10029a342,
  `-[RFGoogleFontImporter run]` / `importGoogleFonts`, around **0x100120f0c** / **0x100121a60**),
  and keeps the files in a library named **"Google Fonts"** (`GoogleFontsLibrary`, sidebar item
  `NavSourceGoogleFonts`, toggle `ShowGoogleFonts`), syncing them gradually ("Google Fonts are being
  synced and will appear gradually as the download completes.").
- FontCraft does **not** call RightFont's private server (it's their service and needs their token);
  it keeps using Google's public font list and stylesheet addresses. Same result for the person: a
  "Google Fonts" library, one list per family.

## Panels

- Main window is an `NSSplitView` (`MainSplitView`, autosaved with a `collapsed` flag) with a
  `sidebarItem` and an `inspectorItem` that can collapse (`setCanCollapse:`, `toggleCollapse:`).
  Widths come from the layout file, not code → FontCraft's 210 pt / 400 pt stay **estimated**.

## Sidebar and Library settings

All `-[RFPreferencesHandler …]` getters read the key and return a fixed value when it is unset:

| Setting | Default | Function | In FontCraft |
|---|---|---|---|
| `showGoogleFonts` | **on** | **0x1000a9abc** | Settings › Library "Google Fonts" — on |
| `showAdobeFonts` | **on** | **0x1000a9b64** | Settings › Library "Adobe Fonts" — on |
| `showStarredFonts` (Favorites in the sidebar) | **on** | **0x1000a9a14** | **new** toggle, on |
| `showSmartFilters` (Smart Lists section) | **on** | **0x1000a9f54** | **new** toggle, on |
| `launchOnLogin` | **off** | **0x1000a89f0** | Settings › General — off |
| `showMenubar` | **off** | **0x1000a8b40** | "keep running in the menu bar" — off |
| `activatingFontsAfterImported` | **off** | **0x1000a9098** | Import Fonts copies without turning fonts on |
| `skipDuplicateFonts` | **off** | **0x1000a8ff0** | not a setting in FontCraft; it never copies over a file with the same name |
| `fontPreviewGroupFontsByFamily` | **off** | **0x1000aa29c** | FontCraft always groups by family (its design; kept) |
| `syncInterval` | **60**, never below 60 (`if value <= 59 → 60`) | **0x1000a87e0** | library cloud sync; FontCraft has none |
| `fontPreviewText` | a fixed string when unset | **0x1000aa6c8** | the English sample "The quick brown fox jumps over the lazy dog" (string 0x1002952e0) is FontCraft's English sample; other languages use their own pangram (person's request) |

- **Smaller size**: `new = max(slider.integerValue − 4, slider.minValue)`
  (`-[RFMainWindowController decreaseFontSizeNotified:]` **0x10003b5d0**: `x22 = value − 4`, then
  `csel` with the slider's minimum). FontCraft: `(size − 4).max(SIZE_MIN)`, same rule.

→ `src/settings.rs` + `src/main.rs`: Settings › Library has "Favorites" and "Smart lists" switches
(both on) that show or hide those sidebar parts.

## Start-up, Auto Activation and quitting

Settings defaults (what each `RFPreferencesHandler` getter returns when the key is unset):

| Settings key | Default | Function |
|---|---|---|
| `EnableAutoActivationFonts` | **on** | `-[RFPreferencesHandler enableAutoActivationFonts]` **0x1000a91e8** |
| `AutoActivateFontFamily` (whole family) | **off** | `autoActivateFontFamily` **0x1000a9470** |
| `AutoDeactivateAutoActivatedFonts` (on quit) | **off** | `autoDeactivateAutoActivatedFonts` **0x1000a9518** |
| `AutoDeactivateManuallyActivatedFonts` (on quit) | **off** | `autoDeactivateManuallyActivatedFonts` **0x1000a95c0** |
| `CanBeOpenedByShortcut` | **off** | `canBeOpenedByShortcut` **0x1000a8a98** |
| `AutoActivationEnabledApps` | none (all apps) | `autoActivationEnabledApps` **0x1000a93e0** |

Other keys seen: `EnableAutoActivationFontsShortcut`, `Silent`.

- **Launch at login**: `-[RFGeneralPreferencesViewController launchAtStartupSwitchButtonClicked:]`
  **0x1000078a8** calls `SMLoginItemSetEnabled("com.rightfontapp.RightFontHelper", on)` (helper id string
  at 0x1002954e7). FontCraft has no helper app inside it, so it uses a LaunchAgent with the same effect.
- **App in front**: `-[RFUIScenesRouter didActivateApplicationNotification:]` **0x1000c0bac** follows
  `NSWorkspaceDidActivateApplicationNotification`. FontCraft checks the front app every 1.5 s (estimated).
- **When the app quits** — `-[RFUIScenesRouter deactivateFontsWhenQuits]` **0x1000c0234**:
  if "auto-activated" is on → `deactivateAutoActivatedFonts` (0x1000725b0); if "manually activated" is
  on → turn those off too; otherwise only the fonts turned on temporarily
  (`deactivateTemporarilyActivatedFonts` 0x100072ea0) go off.
- **Supported apps** (bundle ids, strings 0x10028abb5…0x10028b05e, kept in this order in FontCraft's list):
  Photoshop, Illustrator (+Beta), InDesign, InCopy, After Effects, Animate, Muse, QuarkXPress, Sketch,
  Affinity (all Designer/Photo/Publisher 1 and 2 builds and `com.canva.affinity`), Pixelmator, Swift
  Publisher 5, Framer, OmniGraffle 7, SketchBook, Pages, Keynote, Numbers, iMovie, Final Cut, MindNode,
  Pixelmator Pro, XD, Figma, Word, Excel, PowerPoint, TextEdit, and the newer Apple ids `com.apple.Pages`,
  `.Keynote`, `.Numbers`, `.FinalCutApp`, `.pixelmator`. FontCraft adds VectorCraft, Motion, LibreOffice
  and ONLYOFFICE (person's request).
- **Changing the selected text's font** — AppleScript templates (strings 0x10028b730–0x10028d4fa), now
  used by FontCraft for the same apps:
  - Photoshop: `set font of text object of current layer of current document`
  - Illustrator: `set text font of every character of selection of current document`
  - InDesign / InCopy: `set applied font of every character of selection of active window`
  - After Effects: `DoScript` with JavaScript setting `textDocument.font`
  - QuarkXPress: `set font of story 1 of current box`
  - Word / Excel: `set name of font object of selection of active window`
  - PowerPoint: `set font name of font of text range of selection of active window`
  - Keynote: each selected `text item`/`shape` → its `object text`; Pages: `body text` or a shape's `object text`
  - TextEdit: reads `AXSelectedTextRange` of the text area, then `set font of characters c1 thru c2`
  - Pixelmator: `set font of text content of current layer of front document`
  - Every other app: copy (`keystroke "c" using command down`, then `delay 0.2`, string **0x10028b730**),
    then activate, `delay 0.1`, paste (`keystroke "v"`, string **0x10028b813**). FontCraft swaps the
    copied rich text's font table between the two (estimated: RightFont's own swap step wasn't read).

→ `src/autoact.rs`: the per-app scripts and the 0.2 s / 0.1 s delays (before: Word/Excel only, 450 /
350 / 400 ms guesses). `src/settings.rs`: "turn off auto-activated fonts when FontCraft quits" now
starts **off**, as in RightFont (before: on, estimated).

## Additions requested by the person (not from RightFont)

- **Accents**: VectorCraft's six accents in its order — Coral, Blue, Violet, Green, Amber, Pink — with its
  light-panel and dark-panel tones, from VectorCraft's open-source `crates/ui-egui/src/theme.rs`
  (`Accent::ALL`): coral #d9472f/#f06a52, blue #2563eb/#5b96ff, violet #7146dc/#a98bff,
  green #16955a/#3fcf8e, amber #b06d06/#f5b544, pink #cc3672/#f36aa3.
- **Glyphs page**: glyphs drawn at 48 pt (twice RightFont's default of 24), with no box around each.
- **Start-up**: a login item (`~/Library/LaunchAgents/app.fontcraft.login.plist`) starts FontCraft with
  `--background`; a menu bar icon brings it back full screen; an optional global shortcut does the same.
- **Auto Activation**: fonts are turned on (scope 2, as above) for the app in front; selected text is changed
  with RightFont's AppleScript where it has one and by copy → swap → paste elsewhere (see above; needs
  macOS Accessibility permission). Fonts turned on this way also go off when their document closes
  (apps that report documents: Pages, Keynote, Numbers, TextEdit, Word, Excel, PowerPoint), when the app
  quits, or when FontCraft quits (option).
- **Sources**: Google Fonts (FontCraft's download folder) and Adobe Fonts (Creative Cloud's
  `~/Library/Application Support/Adobe/CoreSync/plugins/livetype/.r`) are sources, not libraries.

## Interaction, menus, previews and systems (person's request, this round)

- Windows: every outside program (app check every 1.5 s, registry, printing…) is started without a
  console window (`CREATE_NO_WINDOW`, `src/paths.rs::command`); before, an empty terminal flashed open
  and shut every 1.5 s.
- Inserting a character keeps the target app's size and style: pasted as plain text, selected back
  (Shift+Left per character), then changed with the app's script or copy → swap font → paste (macOS).
  Fonts turned on for an app show as on (green asterisk); switching one off forgets it.
- Right-click on a library/folder/list: Activate, Add (Import window pointing at it), Delete (confirm;
  a library/folder leaves FontCraft with its files kept, a list's files go to the system Trash). On a
  font in the list: Activate, Delete (to the Trash; system, installed and Adobe fonts can't).
- Glyph hover preview: 144 pt, white on black, accent frame; baseline, x-height, cap height and width
  as fine lines, ascender and descender dotted; Character Info uses it with the names in capitals.
- Google Fonts: hover preview at 48 pt (the family's regular style fetched once); "Download All" for
  the families shown, four at a time, with progress.
- First start on macOS asks for Accessibility once. Minimum systems: macOS 12, Windows 10 22H2,
  Ubuntu 22.04-era Linux, Android 12, on x64 and ARM64.

## Glyph cells, glyph preview and deleting (decompiled this round)

| Rule | Value in FontCraft | Where in RightFont |
|---|---|---|
| Glyph cell corners | radius 8 (`metrics::CELL_RADIUS`) | `-[RFFontGlyphView updateLayer]` 0x1000670f4 (`fmov d0, #8.0`) |
| Chosen glyph background | accent at 0.08 (`SELECTED_TINT`) | same, constant 0x100282548 = 0.08 |
| Glyph under the pointer | text colour at 0.025 (`HOVER_TINT`) | same, constant 0x1002825d0 = 0.025 |
| 144 pt preview placement | advance width centred: x = (W − advance) × 0.5; baseline = descent + (H − (ascent + descent)) × 0.5 (`paint_glyph_metrics`) | `-[RFGlyphDrawingView drawRect:]` 0x10008d54c (CTFontGetAdvancesForGlyphs, CTFontGetAscent/Descent, `fmov d1, #0.5`) |
| Single glyph in a cell | centred on its ink box: x = (W − inkW) × 0.5 − inkX (already so in the grid) | `-[RFGlyphView drawRect:]` 0x10000bc4c |
| Delete asks first | confirmation window before deleting | `-[RFFontPreviewViewController deleteFromLibraryMenuItemClicked:]` 0x100021950, title "Delete Fonts?" 0x10028629d |
| Deleting a library | warns that its fonts go with it | `-[RFLibraryHandler deleteLibrary]` 0x100118468, text 0x10029a6f0 |
| A file already gone | not an error (files that no longer exist are skipped) | `-[RFLibraryHandler deleteFontFile:]` 0x10011f3cc (error code 4 ignored) |

RightFont deletes a library's own copies for good (`removeItemAtPath`, 0x10011f3cc) and moves libraries to the
Trash with Finder (AppleScript at 0x1002873c8). FontCraft keeps the person's choice: removing a library or folder
keeps its files, and deleting fonts or lists moves them to the system Trash so they can be brought back.

## Icons, system look, and apps that read fonts only when they open (this round)

- **Icons** come from the person's drawings (`assets/icons/source/*.svg`, from the folder "Iconos FontCraft"),
  drawn with resvg at every size: macOS `.icns`, Windows `.ico` (app and `.fcl`), Linux PNGs (`.deb`/`.rpm`),
  Android launcher icons (classic PNGs, plus an adaptive icon with a night version and a monochrome layer).
  The window/taskbar/Dock icon and the sidebar logo use the light or dark drawing to match the system
  (macOS `AppleInterfaceStyle`, Windows `AppsUseLightTheme`, GNOME `color-scheme`).
- **System look**: the accent colour follows the system by default (macOS `controlAccentColor`, Windows DWM
  `AccentColor`, GNOME 47 `accent-color`); VectorCraft's six swatches stay as choices. Character Info and
  Similar Shapes are now real system windows like the other dialogs; Character Info opens centred on the screen.
- **Apps that read fonts only when they open** (checked in their programs, see mechanics.json):
  - VectorCraft (Type › Refresh Font List) and DesignCraft (Type › Reload Fonts): when a font is turned on
    while they are open, FontCraft chooses that menu command for them (macOS, System Events; any of the
    command's translations), and says "Font list refreshed in …".
  - LibreOffice for Mac, PhotoCraft, PdfCraft, SolidCraft, TextureCraft: no such command, so FontCraft says to
    reopen them to see the fonts just turned on (fonts turned on before they open are seen: photocraft-cli and
    designcraft-cli list FontCraft's active fonts as installed).
  - TextureCraft and SolidCraft were added to the apps FontCraft works with.
  - Windows and Linux: a turned-on font's file is also placed in the user's font folder
    (`%LOCALAPPDATA%\Microsoft\Windows\Fonts\FontCraft`, unregistered; `~/.local/share/fonts/FontCraft` as a
    hard link when possible), because the ArtCraft apps read font folders themselves.

## System look in RightFont (decompiled this round)

| Rule | Value in FontCraft | Where in RightFont |
|---|---|---|
| Accent colour | the system's (default; `settings::accent_color`, `oslook::system_accent`) | `NSColor.controlAccentColor` (selector 0x100378220, stub 0x100243b60) used by 16 drawing functions: tabs 0x1000be314, sidebar rows 0x1000c61c4, slider knob 0x10003d8b0, font rows 0x100068b4c… |
| Sidebar chosen row | black 6 % light / white 9 % dark (`settings::row_tint`) | `-[RFNavigationItemRowView drawRect:]` 0x1000c61c4, constants 0x1002822a8 = 0.06, 0x100282750 = 0.09 |
| Sidebar row under the pointer | black 2.8 % / white 4 % | same, 0x100282748 = 0.028, 0x1002822a0 = 0.04 |
| Drop target row | 2 px accent outline | same (`fmov d0, #2.0`, controlAccentColor) |
| Tabs | pills: radius = height × 0.5; chosen accent with white text | `-[RFTabButton drawRect:]` 0x1000be314 (`fmul d4, d8, #0.5`; unchosen white 0.92 at 0x100282590; 13 pt) |
| Dark or light | follows the system unless set (default on) | `+[RFGlobals isDarkAppearance]` 0x1000e9858, `-[RFPreferencesHandler enableSystemDefaultAppearance]` 0x1000a8d38 |
| Fonts registered by others | RightFont does nothing (`registeredFontsChangedNotification:` 0x1000ddfbc is empty) | FontCraft goes further at the person's request: it refreshes VectorCraft/DesignCraft or says to reopen apps that read fonts only at start |

## Font requests, Missing Fonts, the font menu and the host's look (this round)

| Rule | In FontCraft | Where in RightFont |
|---|---|---|
| Fonts other apps ask for (macOS) | `fontreq`: CoreText font-request source on the main run loop; a catalog font with the asked PostScript, full or family name is turned on and handed back; it goes off when the asking app quits | `-[RFFontActivator startFontRequestRunLoopSource]` 0x10004ee84, `receiveFontRequest` 0x10004f5b8 |
| Missing Fonts window | `missing`: names apps asked for that FontCraft lacks, and the fonts of the document open in front (read from the file by `docfonts`: Office, OpenDocument, RTF, SVG/HTML) that aren't installed or on; "Activate All" for those in the catalog (on for that document/app), "Replace with…" for the rest | `handleMissingFonts:targetID:appName:` 0x10004faf8; "RightFont detected %lu missing %@ in this document." 0x10028fc70; "%lu missing %@ activated" 0x10028f17b |
| Activate Temporarily | on until FontCraft quits (`temp_active`, turned off in `autoact_quit`) | `deactivateTemporarilyActivatedFonts` 0x100072ea0, "Activate Temporarily" 0x100286413 |
| Export | folder picker titled "Select the destination…", a folder named after the font made inside, files copied (`fcl::export_fonts`) | 0x100021294, `exportFonts:toLocation:` 0x10007e218 |
| Show Location | Finder `open -R`, Explorer `/select,`, Linux FileManager1.ShowItems (`paths::reveal`) | "Show in Finder" 0x1002864d0 |

Font menu (central list), all with drawn icons: Activate / Deactivate, Activate Temporarily; Add to /
Remove from Favorites, Add to Smart List ▸ (the eight smart lists; added families are saved in
`smart-lists.txt`), Add to Library ▸ (every library and folder with its whole tree, `treemenu`); Export,
Show Location; Delete from Library / Delete from Folder (by where the font is).

Import Fonts: "In" offers only Library ▸ / Folder ▸ (nothing chosen in advance), each the whole tree with
icons; both pop-ups are the same control and size; "Add" waits until a place is chosen. A FontCraft library
(.fcl) can be chosen everywhere: on macOS it is a package, so "Choose" uses a panel that takes packages
(`fcl::pick_folder_or_library`); on Windows and Linux the .fcl folder is marked with FontCraft's library
icon (desktop.ini; GNOME custom icon and KDE .directory) so it is recognised as FontCraft's.

Apps: every LibreOffice module on Linux/Windows (Writer, Calc, Impress, Draw, Base, Math; one process,
soffice.bin), WordCraft, DeckCraft, GridCraft, FilmCraft, PotCraft, ONLYOFFICE, and any installed app that
edits text, sheet, slide or graphics documents (macOS: its declared document types; Linux: its desktop
entry's MIME types or categories; Android: the package list). Inserting a glyph moves the caret past a
selection first, so the glyph never replaces selected text. Linux: after turning a font on, the whole user
font folder's cache is rebuilt and its time updated, so open GTK/LibreOffice apps pick it up.

Look: interface text 1 pt larger; Ubuntu's ten accent colours plus the system's own (default); the size
slider stretches over the details panel's width; Google Fonts' hover preview at 60 pt with every letter.
Libraries: eframe/egui 0.37, objc2 0.6, ureq 3, rfd 0.18, tray-icon 0.26, global-hotkey 0.10, resvg 0.49,
zip 9.5; Rust 1.99.1.

## Font requests in detail (decompiled this round)

| Rule | In FontCraft | Where in RightFont |
|---|---|---|
| The system's own UI fonts are never missing | `fontreq::is_system_ui`: names starting with SFUIText, SFUIDisplay, SFCompact, SFProText, SFProDisplay, HelveticaNeueUI (and CoreText's hidden "." fonts) are ignored by the request handler and by the document check | `-[RFFontActivator handleRequestFonts]` 0x10004f89c (six `hasPrefix:` on strings 0x10028c1de–0x10028c214) |
| Each requested name once per app | `told` keyed by app and name | same function (handled-name sets per target) |
| Requested fonts turned on automatically | always on in FontCraft (RightFont's default) | `enableAutoActivationFonts` 0x1000a91e8: true when never set (key EnableAutoActivationFonts) |
| Missing Fonts window shown | always (RightFont's default) | `showMissingFontsAlert` 0x1000a9338: true when never set (key ShowMissingFontsAlert); `handleMissingFonts:targetID:appName:` 0x10004faf8 |

## Fonts off when done, apps that take the font, native menus (this round)

- **Nothing left on in the system.** macOS fonts are registered in the session scope (3), not RightFont's
  persistent scope (2, 0x10004b974); fonts turned on for an app go off when the document closes (any app:
  its windows' AXDocument names, or its document scripting) or the app quits; on quit FontCraft turns off
  every font it turned on (kept-on fonts stay in its list and come back next time); fonts an earlier version
  left on persistently are taken off at start.
- **Changing the selected text's font.** VectorCraft: its "Highlight Font Name" (⌘⌥⇧F), the family typed,
  Return (after its Refresh Font List). DesignCraft: its type.char command through designcraft-cli when it
  listens on its control port, else paste. LibreOffice, ONLYOFFICE and the rest: copy → swap → paste, now
  waiting for the copy (up to 2 s) and keeping sizes (RTF with \fs, else the app's HTML), the clipboard
  restored 1.2 s after pasting.
- **Missing Fonts when a document opens.** Besides AXDocument, the front window's title is matched to its
  file with Spotlight (LibreOffice, ONLYOFFICE).
- **Host look.** Right-click menus (fonts, libraries, folders) and the Import window's pop-ups are the
  system's own menus on macOS (NSMenu with macOS icons) and Windows; drop-down arrows are the system's own
  artwork (SF Symbol, Segoe icon glyph, GTK theme icon). The details panel is never narrower than four glyph
  columns; the top bar is as before.

## Missing-font checks and deactivation (decompiled this round)

| Rule | In FontCraft | Where in RightFont |
|---|---|---|
| One document scan at a time; one asked for meanwhile runs right after | `MissingState::busy` / `next`, `read_document` | `scanUsedFontsWithCommonCode:…` 0x10004dae4 (flags self+0x20 / +0x21), `finishMissingFontScan` 0x10004d564 |
| Stop checking an app after AppleScript errors -600, -609, -610, -1712, -1750, -2741 (asked again when it comes to the front) | `STOP_CHECKING_ERRORS`, `osascript_code`, `stopped_checking` | `shouldStopMissingFontTimerForError:includeTargetNotRunning:` 0x10004d488 |
| Auto-activated fonts turned off on quit | always (the person's rule) | `autoDeactivateAutoActivatedFonts` 0x1000a9518: off by default in RightFont |
| App quit noticed | process list every 1.5 s | `didTerminateApplicationNotification:` 0x1000c0ddc (NSWorkspace notification) |
| Font swapped in copied text | RTF, or HTML when the RTF has no sizes | `changeFontForRTF:` 0x10004c60c (RTF only, on the main thread) |

## Host resources, printing and insertion (this round)

| Rule | In FontCraft |
|---|---|
| Interface icons (asterisk in lime when on and grey when off, favourite star, library, folder, set, family, smart lists, automation, menu actions) are FontCraft's own SVG files, drawn with resvg; on macOS menus the line icons are templates the system tints | `assets/icons/ui/*.svg`, `uiicons.rs`, `nativemenu::MIcon` |
| Sidebar sections (Biblioteca, Orígenes, Listas inteligentes, Bibliotecas, Carpetas) fold with the system's own disclosure arrow; what is folded is remembered | `oslook::section_header`, `sidebar-folded.txt` |
| Colours are the system's: macOS AppKit window/control background, label, secondary label and separator under Aqua/DarkAqua; Windows 11 Fluent; Linux libadwaita | `oslook::system_colors`, `Pal::from_system` |
| Print window: the installed printers, the default one chosen; the paper list and default paper are the chosen printer's own. macOS and Linux: CUPS (`lpstat -e`, `lpstat -d`, `lpoptions -p <queue>` printer-info, `lpoptions -p <queue> -l` PageSize with `*` default, sizes and names from its PPD). Windows: the printer driver (System.Drawing PrinterSettings: InstalledPrinters, PaperSizes, DefaultPageSettings). macOS: the paper chosen in System Settings wins when the printer takes it. No printer: A4, or US Letter where Letter is used | `print::installed_printers`, `papers_of`, `load_printers` (in the background) |
| A double-clicked glyph is typed into the app (keyboard events with the character), so the app adds no spaces around it; pasting is the fallback | `autoact::type_text` |
| Before copying or pasting into an app, FontCraft waits until the app is really in front (up to 2 s, then 150 ms) | `autoact::wait_front` |

## Decompiled this round (RightFont 01f4a698…, built-in engine)

| Rule | Value / formula | In FontCraft | Where in RightFont |
|---|---|---|---|
| Double-clicking a glyph copies it | `[self copy:nil]` | double-click inserts into the target app; with none, copies (`panel.rs`) | `-[RFFontGlyphViewController collectionView:doubleClickedAtIndexPath:]` **0x1000b6504** (selref 0x100378298 = `copy:`) |
| What the glyph copy puts on the clipboard | `[[NSPasteboard generalPasteboard] declareTypes:@[NSPasteboardTypeString] owner:nil]` then `setString:<glyph> forType:` — the plain character only, no rich text, no spaces | the glyph is typed (`type_text`); the fallback paste puts only the plain glyph (`pb_set(None, text)`) | `-[RFFontGlyphViewController copy:]` **0x1000b7208** (selrefs 0x1003786b8 `declareTypes:owner:`, 0x10037ef10 `setString:forType:`) |
| Glyph size steps | `+2` while `size ≤ 126.0` (const **0x100282518**), `−2` while `size ≥ 58.0` (0x404d000000000000) | `mechanics::GLYPH_SIZE` range and step | `increaseFontSize:` **0x1000b7388**, `decreaseFontSize:` **0x1000b7300** |
| Copy/paste keystrokes go to the app's own process | `tell application "System Events" to tell application process "%@" to keystroke "c" using command down` + `delay 0.2`; paste: `activate`, `delay 0.1`, `keystroke "v"` to the process | `autoact::keystroke(bundle, key)` addresses the process by bundle id (falls back to the front app); before that FontCraft waits until the app is in front (`wait_front`, ≤ 2 s) | strings **0x10028b730**, **0x10028b813** (refs 0x10031f488, 0x10031f4c8) |
| Apps with their own route | Photoshop, Illustrator (+Beta), InDesign, InCopy, After Effects, Muse, QuarkXPress, Sketch (+Beta), Affinity (1, 2, Canva), Pixelmator (+Pro), Swift Publisher 5, Framer, OmniGraffle 7, SketchBook, Pages, Keynote, Numbers, iMovie, Final Cut, MindNode, XD, Figma, Word, Excel, PowerPoint, TextEdit | `autoact::APPS` (Sketch Beta and Adobe Muse added this round) | bundle-id strings 0x10028abb5–0x10028b05e, 0x10028c6b5 |
| Glyph drawing view | background `NSColor.textBackgroundColor`, glyph `textColor`; glyph centred: x = (w − advance)·0.5, baseline = descent + (h − (ascent + descent))·0.5 | centring used in `paint_glyph_metrics`; the background stays black, as the person asked | `-[RFGlyphDrawingView drawRect:]` **0x10008d54c** (selrefs 0x10037fe50 `textBackgroundColor`, 0x10037fe58 `textColor`) |
| Toolbar slider knob | fill `controlBackgroundColor`, stroke 2.0 `secondaryLabelColor`; knob 10.0 (small) or 12.5 across, inset 4.0/3.5 or 2.5/0.5 | not copied: egui draws the knob in its rail colour (circle, system accent rail) | `-[RFToolbarSliderCell drawKnob:]` **0x100135fa8** (selrefs 0x100378228, 0x10037d470) |
| System colours used | `windowBackgroundColor`, `controlBackgroundColor`, `textBackgroundColor`, `labelColor`, `secondaryLabelColor`, `separatorColor`, `controlAccentColor` | `oslook::system_colors` reads the same AppKit colours under Aqua / Dark Aqua | selector strings 0x1002ee6d7, 0x1002ccf2b, 0x1002eb942, 0x1002d7c1a, 0x1002e17f5, 0x1002e1c06, 0x1002ccf18 |
| Printing | none: RightFont has no print command (no NSPrintInfo / print functions; only `application:printFile:` of the app delegate protocol) | FontCraft's own Print window (printers and papers from the system, see above) | function search "print/paper" **ev_ff27e42a…** |

## Permissions, insertion, arrows, previews and speed (this round)

| Rule | In FontCraft |
|---|---|
| The Mac app is signed with a fixed designated requirement, `designated => identifier "app.fontcraft"`, not the code's hash: Accessibility and Automation permissions survive rebuilds, and the app is listed once (an ad hoc signature's requirement was `cdhash …`, so each build was a new app to macOS) | `packaging/macos/build-dmg.sh` |
| After a glyph is typed (and its font set) the cursor ends right after it: the focused text's AXSelectedTextRange (`{first, last}`, 1-based, the form of RightFont's TextEdit script 0x10028d1ab) and AXNumberOfCharacters are read before and after; a selection left open is closed with →, spaces the app added are taken out, and a cursor that went past is moved back. Apps that don't expose their text (canvas text) get → only when their own script kept the selection, never after a paste | `autoact::ax_caret`, `settle_after_insert`, `LAST_ROUTE` |
| Every fold arrow (sidebar sections, libraries and folders, font families, details panel) is the system's chevron (SF Symbol chevron.right/down; Segoe glyph E76C/E70D; GTK pan-end/pan-down), 9 pt; on library and folder rows it sits on the left, then the icon (at 27 pt) and the name (at 42 pt) | `oslook::disclosure` |
| The Google Fonts preview opens in a small borderless window outside the dialog, on its left, at the hovered row's height; it takes no clicks or focus | `oslook::float_beside`, `oslook::dialog_origin` |
| Reading every font again (≈ 1.6 s for 12,000 styles) runs in the background after a folder changes or a download ends; nothing is read again at start (the first reading includes the Google Fonts folder) | `App::rescan_soon`, `finish_rescan` |
| A choice in the system's right-click menu runs as soon as the menu closes; menu icons are drawn from their SVG once and kept | `open_native_menu` → `native_menu_tick`, `nativemenu::image` cache |

## Decompiled this round: permissions, watcher, popovers, disclosure

| Rule | Value / formula | In FontCraft | Where in RightFont |
|---|---|---|---|
| Accessibility is checked at the moment of each copy, paste or text read, never ahead | `AXIsProcessTrustedWithOptions(@{kAXTrustedCheckOptionPrompt: kCFBooleanTrue})` (GOT 0x10030c230, 0x10030c4f8; dictionaryWithObjects:forKeys:count:1); untrusted → the action is skipped and macOS shows its own request; RightFont opens no settings page itself | `autoact::trusted_or_ask` before every copy/paste and glyph insertion; the "needs access" notice no longer opens System Settings; at first start only macOS's own request, and only when not yet allowed; the Settings pane opens only from the buttons the person presses | `-[RFFontActivator runCopyOnProcess:]` **0x10005191c**, `runPasteOnProcess:` **0x100051a38**, `fontAtTE:` **0x100052398** |
| Re-reading after files change | one-shot timer `dispatch_time(0, 0x11e1a300)` = **300 ms**, leeway `0x2faf080` = 50 ms, restarted by each event; FSEvents latency 0.25 s | `watch::DEBOUNCE_S = 0.3` (was 1.0 s), then a background re-read | `-[RFFileWatcher scheduleResolve]` **0x10000c794**, `flushEvents` 0x10000c8c4, `handleRawEvents:…` 0x10000c5e4 |
| Popovers beside a list | `setBehavior:1` (NSPopoverBehaviorTransient: closes on any outside interaction), `showRelativeToRect:ofView:preferredEdge:2` (NSRectEdgeMaxX, the side away from the list it belongs to); the font preview popover may detach (`popoverShouldDetach:` → YES, `detachableWindowForPopover:` → nil) | the Google Fonts preview is a transient window outside its dialog (gone when the pointer leaves the row); on the left, as the person asked | `showUnavailablePopoverFor:anchor:` **0x100134f80** (call 0x100135684), selref 0x10037d8a8; **0x1000246c8**, **0x1000246d0** |
| Disclosure arrows | sidebar: `shouldShowOutlineCellForItem:` → YES always; an item is expandable when it isn't a leaf kind and has children (`count != 0`); font list: a family is expandable with more than one style (`count > 1`) | `oslook::disclosure` drawn only for libraries/folders with lists (`has_sets`) and families with more than one style | **0x10013049c**, **0x10012e888**, **0x1001086c4** |

## App compatibility, menu speed, app icon and system language (this round)

| Rule | Value / formula | In FontCraft | Where it comes from |
|---|---|---|---|
| Apps that read the font folders themselves don't see fonts only registered with CoreText | ArtCraft apps use fontdb (folders `/Library/Fonts`, `/System/Library/Fonts`, `/Network/Library/Fonts`, `~/Library/Fonts`; VectorCraft: "Refresh Font List — scan the system font folders again"); Inkscape, Scribus, GIMP, Krita use fontconfig (same folders) | on macOS a font turned on is placed in `~/Library/Fonts/FontCraft` (hard link on the same disk, else a copy; macOS and these apps skip symbolic links) and that file is registered with CoreText (session scope), the original stays unregistered: one registration, seen at once by CoreText apps and as an installed font by the rest; turning off unregisters and takes it out; at start `activate::sweep` takes out anything a crash left | `activate::mac::set`, `sweep`; observed: a hard link in that folder was registered by macOS within 8 s, a symbolic link never was |
| RightFont activation | `CTFontManagerRegisterFontsForURL(url, 2)` after unregistering an earlier registration; no file placed in any font folder (so RightFont can't reach fontdb/fontconfig apps either) | FontCraft goes further (folder + one registration), session scope as the person asked | `-[RFFontActivator activateFontByName:andURL:andEnabled:]` **0x10004b974** (calls 0x10004bef4 unregister, 0x10004bf08 register scope 2) |
| VectorCraft's font route | `type.highlightFontName` = **Cmd+Alt+Shift+F**: opens the Character panel's font family search; typing filters, Enter applies to the selected type | kept (`apply_in_app`); works again now that the font is in a folder VectorCraft reads | VectorCraft command table (binary strings) |
| Font-list refresh items | VectorCraft "Refresh Font List" / "Actualizar lista de fuentes", DesignCraft "Reload Fonts": looked for in the menu bar and one submenu deep | `reload_fonts_menu` | current VectorCraft (2026-10) has no such menu item: its menu bar is Apple, VectorCraft, Archivo, Edición, Objeto, Texto, Selección, Efecto, Ver, Ventana, Ayuda |
| ArtCraft control channels | `VECTORCRAFT_CONTROL_PORT`, `DESIGNCRAFT_CONTROL_PORT`, `PHOTOCRAFT_CONTROL_PORT` (+ token file) or `--control <port>`, only at launch; no preference turns them on | not used yet (it would need a change to the session's environment) | binary strings |
| Right-click menu size | the system menu took **2.0 s** to build with 3,575 entries (3,252 lists of one library + 323 of another, each with an icon); the font data took 3.7 ms | a library with more than **40** lists (`MAX_LISTS_IN_MENU`) shows "Nivel principal" and "Elegir lista…", which opens a window with a search over its lists; the menu now builds in **11 ms**; the egui menus draw only the rows in sight | `menus::font_menu_native`, `list_pick_window`, `treemenu::tree`; measured with `FONTCRAFT_MENU_BENCH=1 --screenshot` |
| App icon container | Apple's grid: body 824/1024 of the canvas, corners 22.5% of the body, shadow below, gradient light #FFFFFF→#E9E9EE (edge black 12%) or dark #3A3A3C→#1C1C1E (edge white 15%), drawing 74% of the body | `appicon::render_tile` for the .icns, .ico, Linux PNGs and window icon; while running the Dock (macOS) or window icon follows the system's look, checked every 2 s (`follow_icon_look`, `set_dock_icon`) | the drawing had no container: Finder put it in the system's tile, the Dock showed it bare |
| System panels in the system's language | `CFBundleDevelopmentRegion` en, `CFBundleAllowMixedLocalizations`, `CFBundleLocalizations` (the 25 languages) and one `<lang>.lproj` each | `packaging/macos/Info.plist`, `build-dmg.sh`; NSBundle now prefers `es` on a Spanish system, so the Print panel is in Spanish | without them macOS ran FontCraft in its development language (English) |

## Decompiled this round: adding to lists, activation details

| Rule | Value / formula | In FontCraft | Where in RightFont |
|---|---|---|---|
| "Add to Fontlist" from a font's right-click menu is one item, not a submenu of every list | collects the selected fonts' files (items of type 1), `componentsJoinedByString:`, hands them over with `updateAddedToFontlistFonts:`; the place is chosen in a dialog | a library or folder with more than 40 lists shows "Elegir lista…", which opens the list chooser with a search (`list_pick_window`); small ones keep their lists in the menu, as the person asked earlier | `-[RFFontPreviewViewController addToFontlistMenuItemClicked:]` **0x100021694** (selrefs 0x100377e20, 0x1003801c0) |
| The list chooser | built recursively, each nested level one indentation level deeper (`level + 1`), icons 16 × 16 (18 × 16 for some) | chooser rows indented 12 pt per level (`PICK_INDENT`), icons 16 pt (`PICK_ICON`) | `-[RFLibrarySelectDialog buildFontlistMenu:withNavigationItem:andIndentationLevel:]` **0x10015ee30** |
| Sidebar menu | filled when it opens (`menuNeedsUpdate:` reads the clicked row; no row → `removeAllItems`) | the system menus FontCraft uses (muda) can't fill lazily, so long submenus are avoided instead | `-[RFNavigationViewController menuNeedsUpdate:]` **0x10012e228** |
| Turning a font on | font collections: `CTFontManagerRegisterFontDescriptors(descs, 2, true)` for the named faces; other files: `GetScopeForURL` → unregister from that scope → `CTFontManagerRegisterFontsForURL(url, 2)` → **`CTFontManagerEnableFontDescriptors(descs, enabled)`**; off: `UnregisterFontsForURL(url, GetScopeForURL(url))` | registration as before (session scope, from `~/Library/Fonts/FontCraft`), and now its faces are enabled after registering, so a font disabled in Font Book works while FontCraft keeps it on | `-[RFFontActivator activateFontByName:andURL:andEnabled:]` **0x10004b974** |

## Decompiled this round: version and system compatibility (for releases)

| Rule | Value / formula | In FontCraft | Where in RightFont |
|---|---|---|---|
| Untested macOS | compatible while `operatingSystemVersion.majorVersion < 28` (`cmp x8, #0x1c; cset w0, lt`); otherwise "There may be compatibility issues with the %@ of macOS, it is recommended to upgrade to the latest version of RightFont." with the system version | `mechanics::UNTESTED_MACOS_MAJOR = 28`, `is_compatible_macos`; on a macOS 28 or later FontCraft says so once per system version (in the person's language, `K::UntestedSystem`), remembered in `untested-system.txt` | `+[RFGlobals isCompatibleSystemVersion]` **0x1000e97f0**, shown from **0x100019e58** (string 0x100285eb3), `+[RFGlobals systemVersion]` 0x1000e96b4 |
| The app's version | read from `CFBundleShortVersionString` of the bundle | `build-dmg.sh` writes `CFBundleShortVersionString` and `CFBundleVersion` from `Cargo.toml`, so the GitHub Actions release, its file names and the app report the same number | `+[RFGlobals appVersion]` **0x1000e95f0** (string 0x100296281) |
| Updates | "Check for Updates…" menu item and an "automatically check" preference (an updater framework) | not built: releases come from GitHub (`binaries.yml`); an in-app update check would need a published feed | `-[RFAppDelegate checkForUpdatesClicked:]` 0x1000dbbe8, `-[RFUpdatePreferencesViewController automaticallyCheckForUpdatesCheckButton]` 0x1000900ac |

## Windows and Linux: printing and working with other apps (this round)

| Rule | In FontCraft |
|---|---|
| Print: the system's own dialog prints FontCraft's pages (2 px per point, scaled to the printer's resolution): Windows comdlg32 `PrintDlgW` + `StartDoc`/`StretchDIBits` (offset by the printer's physical margins); Linux GTK `GtkPrintOperation` (its own GTK thread), page masks drawn with cairo. Fallback: the PDF in the default viewer | `printsys.rs`, `print::print_pdf` |
| Windows: a glyph is typed into the app's window (found by its program name, brought to the front after an Alt tap) with `SendInput` + `KEYEVENTF_UNICODE`; a font change copies (Ctrl+C), swaps every font in the copied RTF (else "HTML Format") and pastes (Ctrl+V), then the person's clipboard comes back after 1.2 s; the font is named by family | `winauto.rs` |
| Linux (X11): `xdotool search --onlyvisible --class <app> windowactivate --sync`, then `xdotool type`; Wayland can't type into other apps, so the glyph stays on the clipboard. The .deb recommends xdotool | `winauto::insert` |
| GPU limits | the graphics device asks for the adapter's own limits (2D textures up to 8192): small ARM GPUs refused the default request (`max_texture_dimension_3d` 2048 > 512) | `main.rs`, `WgpuSetupCreateNew::device_descriptor` |
| macOS activation | the original file is registered with CoreText at once (RightFont 0x10004b974; what Affinity and the Apple apps used), and a hard link is left in `~/Library/Fonts/FontCraft` for the apps that only read the font folders (VectorCraft) | `activate::mac::set` |

## This round: Affinity, Finder sidebar, app icon tile, top bar order

| Rule | Value / formula | In FontCraft | Where |
|---|---|---|---|
| Answering another app's font request | the request block reads `NSFontNameAttribute` only when `CFGetTypeID(value) == CFStringGetTypeID()`, hands the name on and returns **nothing** (`mov x0, #0x0` at 0x10004f318); the font is turned on afterwards on the main thread | `fontreq::mac::answer` returns NULL and turns the font on in a worker thread. Before, FontCraft returned font *descriptors*: macOS treats the answer as file URLs (`XTIssueSandboxExtensionsForURLs` → `CFURLCopyResourcePropertyForKey`) and crashed FontCraft (`doesNotRecognizeSelector`, crash reports of 15 Nov 2026 20:37, 20:43, 20:55) whenever Affinity opened a document with fonts that weren't on | RightFont block **0x10004f10c**, `-[RFFontActivator handleRequestFonts]` **0x10004f89c** |
| Affinity sees new fonts | Affinity 3.3.3 lists fonts with `CTFontManagerCopyAvailablePostScriptNames` (liblibkernel) and refreshes on `NSFontSetChangedNotification` (`fontSetChanged:`, libcocoaui), which session-scope registration triggers | nothing extra needed once the request no longer crashes | Affinity's own libraries (imports) |
| Sidebar (Finder, macOS 26) | floating card inset 8 pt, radius 14, soft shadow; titles in sentence case, 11 pt semibold grey, chevron only on hover at the right; rows 28 pt, SF Symbols in the accent colour, name in the label colour, grey rounded selection (radius 8) | `finder.rs` (estimated from Finder's look) | — |
| App icon tile | superellipse `|x|^5 + |y|^5 = 1`, 824/1024 of the icon, shadow 12/1024 down, art 66 % of the tile | `appicon::apple_squircle`, `render_tile` (the old curve cut the top corners) | Apple icon template |
| Top bar | search (SF Symbol magnifying glass) \| "Frase" + phrase field (what is typed becomes the phrase "Frase" brings back) \| Aa ABC abc 123 \| grid, list \| size, target app, light/dark, print, settings | `App::toolbar` | the person's request |

## This round: Affinity behaviour, sidebar type, panels, grid view, icon edge

| Rule | In FontCraft |
|---|---|
| A font another app asks for (macOS font request) | no longer turned on silently: the Missing Fonts window opens with it ("Activate" for catalog fonts, "Replace with…" for the rest), and fonts turned on there stay on while that app runs (`fontreq::handle`, `App::add_requested`, `MissingState::for_app`) |
| Turning such a font off by hand | stays off: each requested name is handled once per run of the asking app (keyed by its process id), so Affinity asking again doesn't bring it back |
| Inserting a glyph by double-click | after typing and selecting it, the font change pastes exactly the glyph in the font at the size the app copied (`apply_in_app_exact`, `rtf_exact`), never the app's copied HTML/RTF with its surrounding spaces; `rtf_for` now uses `\uc1` so its "?" fallbacks aren't pasted as text |
| Sidebar type | names 13 pt, titles 11 pt bold, counts 11 pt, written by AppKit (`oslook::sys_text`): egui's copy of SF wrote "Todas las fuentes" 93.8 pt wide at 13 pt, Finder's engine 106.1 pt |
| Panels | the details panel is the same floating card, in the same colour, as the sidebar; the centre keeps Finder's content colour |
| Views | grid (Font Book tiles: "Aa" in the font, name, styles; sized by the size slider) \| samples (the rows of before) \| list; saved as `tiles` / `grid` / `list` |
| App icon edge | macOS 26 glass rim inside the outline (2.5, 7 and 16 of 1024 px, brightest top-left), bottom edge slightly darker; the light plate is a pale grey (#F6F6F8 → #E4E4E8) so the rim shows |

## This round: Finder sizes, Import icons, Affinity pasting, Missing Fonts in front

| Rule | In FontCraft |
|---|---|
| Sidebar size | follows System Settings › Appearance › sidebar size (`NSTableViewDefaultSizeMode`, read every 5 s; this Mac: 3 = large): names 11/13/15 pt, titles 10/11/12 pt bold, counts as titles, rows 24/28/32 pt (estimated) |
| Sidebar icons | SF Symbols drawn at the text's point size, large scale (`oslook::symbol_for_text`); libraries, folders, their subfolders and families use the same pictures as the Import Fonts window (`uiicons`: library, folder, set, family) at 1.35 × the text size |
| List view | first column titled "Estado" (54 pt) |
| A font just turned on for the app | the change waits until the system finds the font by its PostScript name (up to 3 s) and 0.7 s more, so Affinity has its new font list before the name arrives (it gave the text another font when the name came first) |
| Copied object, not text | Affinity copies a selected text box as SVG markup in plain text: then nothing is pasted (the clipboard comes back) and the person is asked to select the text |
| Affinity rich text | always RTF (it reads PostScript names there), never the HTML route |
| Glyph by double-click | pasted as the system's own RTF for exactly that glyph at the copied size (`cocoa_rtf`; checked by a test that reads it back as one character) |
| Missing Fonts window | floats above every app and takes the keyboard when it opens (`oslook::dialog_to_front`); each font in the catalog has "Activar" and "Reemplazar por…", each one not in it "Reemplazar por…" |

## This round: whole text boxes, menu bar asterisk, instant glyph preview with its keys

| Rule | In FontCraft |
|---|---|
| A whole text box selected (Affinity) | its copy is SVG markup, so FontCraft presses T (Text tool: the box goes into text editing), ⌘A, copies that text, swaps the font in it and pastes it back, then Esc and V (Move tool, box still selected). If that still gives no text, nothing is pasted (estimated key sequence; untested here) |
| Menu bar | the activation asterisk (`uiicons` "asterisk") at 18 pt; a template image (the menu bar's own colour, System Settings › Appearance) while FontCraft works in the background, the activation green while its window is in front; click: open full screen / back to the menu bar |
| Missing Fonts while in the menu bar | only that window comes up (FontCraft's windows unhidden without activating, the main window kept hidden), above the app; it goes back to the menu bar when it closes |
| Glyph hover preview | shown at once (`Response::show_tooltip_ui`), not after egui's 0.5 s tooltip delay |
| Keys for a character | `keys.rs`: the keyboard layout's 'uchr' data through UCKeyTranslate for keys 0–50 with none, ⇧, ⌥, ⌥⇧, and one dead-key step ("⌥E, E"); shown at the top right of the hover preview (15 pt) and of Character Info (17 pt) in the activation green — 60 % less than the 37.5 / 42.5 pt before, as the person asked |

## This round: the menu bar asterisk's two clicks, starting hidden, switching to the target app

What RightFont does (its code, selectors read from its `objc_msgSend` stubs):

| Rule | In RightFont |
|---|---|
| Menu bar click | `-[RFUIScenesRouter toggleMainWindow:]` **0x1000c0614** (the status item's action, set in `showMenubar` **0x1000c0438**: `statusItemWithLength:-2`, image 16 × 16 pt): main window not visible → `showWindow:`, `[NSApp setActivationPolicy:0]` (Regular, 0x1000c0690), `activateIgnoringOtherApps:YES`, `makeKeyAndOrderFront:`; visible → `[mainWindowController close]` |
| Leaving the Dock | `-[RFMainWindowController windowWillClose:]` **0x10003b030**: with "show in menu bar" on, `[NSApp setActivationPolicy:1]` (Accessory: no Dock icon, only the menu bar item) |
| Started at login with the menu bar on | `-[RFUIScenesRouter showMainWindow]` **0x1000bec2c**: `setActivationPolicy:1` at 0x1000bee24 and no window |
| Opened again with no window up | `-[RFAppDelegate applicationShouldHandleReopen:hasVisibleWindows:]` **0x1000db7fc** shows the main window |

What FontCraft does now (macOS; `background.rs`, module `mac`):

| Rule | In FontCraft |
|---|---|
| Start with "Mantener FontCraft en la barra de menús" on (or `--background`) | no window and out of the Dock from the first moment (winit `with_activation_policy(Accessory)`; the window eframe shows after its first frame is see-through and put away at once); only the asterisk |
| Click while FontCraft is the active app with its window up | to the background: a full-screen window goes see-through (alpha 0) and leaves full screen (`toggleFullScreen:`), and FontCraft's windows are put away when macOS posts `NSWindowDidExitFullScreenNotification` (+ 50 ms), at most `LEAVE_FULLSCREEN_MAX_S` = 3 s later; out of the Dock (Accessory) while the asterisk is kept, then `[NSApp hide:]` so the app that was in front before comes back. Measured: click → put away in 0.7 s, the window invisible from the click |
| Click otherwise (hidden, or another app in front) | back in the Dock (Regular), the window up, key and in front, FontCraft active; full screen asked of macOS 0.15 s later (`FULLSCREEN_LOOK_S`), looked at every 0.3 s up to 12 times (`FULLSCREEN_LOOKS`), asked once more (`FULLSCREEN_ASKS` = 2) when macOS starts the move and gives it up (willEnter without didEnter for 1.5 s, `FULLSCREEN_MOVE_STALE_MS`; seen on this Mac: willEnter 12094 ms, second ask 13689 ms, didEnter 14271 ms) |
| Where the clicks are handled | inside the status item's own click handler, with AppKit's calls. eframe 0.37 runs FontCraft's screen code (`App::ui`) only while the window is on screen; while it is hidden or covered it runs `App::logic` alone, so the old trip to the menu bar, which waited for the next screen pass, never finished: the window was left out of full screen, small |
| Opened again (Finder, Dock, Launchpad, Spotlight) while in the menu bar | FontCraft becoming the active app with nothing on screen, at least 2.5 s after a trip (`REOPEN_SETTLE_MS`), brings the window up full screen 0.35 s later (`REOPEN_DELAY_S`); not for the Missing Fonts window |
| Asterisk colour | the activation green while FontCraft is active with its window up (also while macOS animates it into full screen), the menu bar's colour otherwise |
| Background work while hidden | `App::logic`: the asterisk and the shortcut, fonts going off when their app or document closes, other apps' font requests → the Missing Fonts window (its pass is asked for with `ViewportCommand::RequestPaintWhileHidden`) |
| Target app's icon in the toolbar | the app comes to the front first, at once: `[NSApp yieldActivationToApplication:]` (macOS 14 and later) then `-[NSRunningApplication activateWithOptions:]` (all windows); not running → `open -b` on another thread; then the selected font goes to it on its own thread. Measured: 5–10 ms (Finder as the probe). Before: `osascript … activate` waited on the main thread, where FontCraft also answers the apps' font requests (`fontreq.rs`, `CFRunLoopGetMain`), so FontCraft froze ("se queda atascado") |
| Check | `FONTCRAFT_TEST_MENUBAR=<log>` (with `FONTCRAFT_TEST_CLICKS=driver` for real clicks posted on the asterisk) logs the window's state every second and each trip's steps |

## This round: CorelDRAW and TraceDraw in Automation, the Dock icon, smaller keys

| Rule | In FontCraft |
|---|---|
| Keys for a character | 17 pt in Character Info, 15 pt in the Glyphs page's preview (60 % less than before, as the person asked) |
| CorelDRAW (macOS) | listed by its suite-year bundle id (`com.corel.coreldrawsuite.<year>.coreldraw`, this Mac 2026 / 27.2.0.135), found two folders deep (`/Applications/CorelDRAW Graphics Suite/27/`); Corel PHOTO-PAINT the same (`….photopaint`) |
| CorelDRAW's font change | no AppleScript, no accessibility for its windows (only its menus answer), no font message like Affinity's, and its help says the font is chosen from the property bar's Font list. FontCraft copies the selection: text selected with the Text tool comes as rich text, its font is swapped and pasted back; a whole selected object comes as CorelDRAW's own format + PNG + TIFF + its plain words ("Ancizar\0", no RTF, seen on this Mac), so nothing is pasted (no second copy of the object) and the person is asked to select the text |
| TraceDraw (`dev.tracedraw.app` 0.48.0) | listed; no AppleScript, no accessibility for its window, its agent server off (`mcp_window: false` in its settings), no font message; it copies a selected object as SVG text (a converted shape had no font at all), so FontCraft only turns the font on for it. It reads the installed fonts when it opens (no font-change listener), so a font turned on while it is open shows after reopening it |
| Dock icon | each time FontCraft comes back from the menu bar into the Dock, macOS makes a new Dock tile (the generic "exec" icon for a program started outside a .app, as Craft Anything starts the build): FontCraft sets its own icon again on the next pass (`background::take_dock_icon_due`) |

## This round: glyph preview beside the details panel, TraceDraw's font list

| Rule | In FontCraft |
|---|---|
| Glyph preview (pointer on a glyph in the Glyphs page) | a 300-point drawing at 144 pt with the font's lines, in a 6-point frame, its right edge against the details panel's left edge and centred on the panel's height (the person's request), in its own layer above everything (`App::glyph_preview`, `details_card`), no longer a tooltip by the pointer |
| How TraceDraw finds fonts | `fontdb`: it scans /System/Library/Fonts, /Library/Fonts, ~/Library/Fonts (and Adobe's folder) once when it opens ("loaded 933 font faces… system fonts read in 144 ms" in its log), no folder watcher, no CoreText notice, no rescan command; its Font Manager installs files only by hand |
| A font turned on for TraceDraw | FontCraft keeps its file in ~/Library/Fonts/FontCraft (scanned by TraceDraw), so TraceDraw lists it the next time it opens; FontCraft says to reopen TraceDraw. The font stays 60 s after TraceDraw closes (`REOPEN_GRACE_S`, estimated) so reopening it finds the font, then goes; it also goes when FontCraft quits. TraceDraw doesn't show its documents to other apps (its window isn't in macOS accessibility), so a closed document can't be noticed by itself |

## This round: DesignCraft through its control channel

| Rule | In FontCraft |
|---|---|
| DesignCraft's channel | DesignCraft 0.20.1 (`ai.storyteller.designcraft`) listens on 127.0.0.1 only when it opens with `DESIGNCRAFT_CONTROL_PORT` set (or `--control PORT`): "designcraft: control server listening on 127.0.0.1:<port>". Its `designcraft-cli script - --connect PORT` runs one `{"command": id, "params": {…}}` per line in it and answers `{completed, results}` (or `{error: "…select text or a text frame"}`); not listening: nothing on its output |
| Opening the channel | when FontCraft starts working with apps (macOS) it sets `DESIGNCRAFT_CONTROL_PORT=7979` for the apps macOS opens from then on (`launchctl setenv`; a value already set is kept) and takes it back when it quits (`launchctl unsetenv`). A DesignCraft open before that has no channel until it is reopened; meanwhile the routes of before are used (its Reload Fonts menu, copy → swap → paste) |
| Font on the selection | `type.char {attrs: {fontFamily, fontStyle}}` with the face chosen in FontCraft, in the background, FontCraft staying in front: the selected text, or every character of the selected text frames (sample: frame 21 → 27 characters in Georgia Regular, live in a test DesignCraft and headless). Nothing suitable selected: "Select the text first" |
| A font just turned on | `font.reload` first (DesignCraft reads its font folders again: "cataloged 1279"), so it is in its font menu at once; the same after the Missing Fonts window turns fonts on |
| Glyph by double-click | `text.insert {text, raw}`, `text.move {dir: left, extend}` once per character, `type.char`, `text.move {dir: right}`: exactly the glyph in the face (sample: 1 character) |
| Font off when its document closes | `document.list` (titles, the active one first) names DesignCraft's open documents; a font turned on for a document goes when it closes, and follows it when it is saved under a new name (same count, one title swapped) |
| Missing Fonts | a DesignCraft document shown for the first time is asked `font.list`; the families marked `missing` open the Missing Fonts window for DesignCraft |
| Waiting | at most `CHANNEL_WAIT_S` = 5 s for an answer (estimated; it answered at once) |

## This round: PhotoCraft through its control channel

| Rule | In FontCraft |
|---|---|
| PhotoCraft's channel | PhotoCraft 0.23.1 (`ai.storyteller.photocraft`) listens on 127.0.0.1 when it opens with `PHOTOCRAFT_CONTROL_PORT` set, and answers only after `{"method":"auth","params":{"token":…}}` with its control token (64 hex digits, from `PHOTOCRAFT_CONTROL_TOKEN` or the file `PHOTOCRAFT_CONTROL_TOKEN_FILE` names; a file readable by others is refused: "restrict it to its owner"). Requests are JSON lines `{"id","method","params"}`; `engine.execute {command, params}` runs a command (`photocraft-cli serve`'s protocol) |
| Opening the channel | when FontCraft starts working with apps (macOS) it keeps its own token in `photocraft-control-token` (owner-only, made once from /dev/urandom) and sets `PHOTOCRAFT_CONTROL_PORT=7981` and `PHOTOCRAFT_CONTROL_TOKEN_FILE` for the apps macOS opens from then on (`launchctl setenv`; values already set are kept and used), taking back what it set when it quits. PhotoCraft runs once on a Mac (a second copy closes itself), so one open before FontCraft has no channel until reopened; meanwhile the routes of before are used |
| Font on the selection | `type.setStyle {font, fontStyle}` with the face chosen in FontCraft, in the background: all the text of the selected type layers, size kept (checked: Helvetica 48 pt → "Georgia Regular"); with no type layer selected PhotoCraft answers "select a type layer" → "Select the text first". A font just turned on is waited for until the system lists it (PhotoCraft asks CoreText: `CTFontCollectionCreateFromAvailableFonts`) |
| Font off when its document closes | `session.list` names the open documents (the active one first); a font turned on for a document goes when it closes, and follows it when it is saved under a new name |
| Missing Fonts | a PhotoCraft document shown for the first time is asked `type.resolveMissingFonts {}`; the families it lacks open the Missing Fonts window for PhotoCraft |
| Glyphs | PhotoCraft's commands have no "type at the text cursor" (type.edit needs character positions), so a double-clicked glyph is typed into it as before |
| Check | `FontCraft --test-photocraft-channel PORT TOKENFILE` against `photocraft-cli serve --port PORT --control-token-file TOKENFILE` |

## This round: PhotoCraft's font list, channels kept for the session, preview keys

| Rule | In FontCraft |
|---|---|
| PhotoCraft reads fonts once | it lists the installed fonts when it opens and never again (tested with photocraft-cli serve: a family registered with macOS after it started → `type.fonts` gives no faces and `type.setStyle` warns "font '12SaruYellowFog' is not installed; text may render using a fallback face"; a PhotoCraft started after the font was on lists it and sets it exactly). That is why it "changed fonts a couple of times" (fonts it already had) and then not (fonts FontCraft had just turned on) |
| A font PhotoCraft hasn't read | FontCraft asks `type.fonts {family}` first; with no faces nothing is set (the layer doesn't take a font drawn with a stand-in) and FontCraft says to quit and reopen PhotoCraft; the font stays on (and 60 s after PhotoCraft quits) so the reopened PhotoCraft has it |
| Channel variables | `DESIGNCRAFT_CONTROL_PORT`, `PHOTOCRAFT_CONTROL_PORT` and `PHOTOCRAFT_CONTROL_TOKEN_FILE` stay set for the rest of the login session when FontCraft quits (macOS forgets them at logout): taking them back left a PhotoCraft or DesignCraft reopened while FontCraft was closed or restarting without its channel |
| Keys in the glyph preview | 18 pt (20 % more than 15 pt, as the person asked); Character Info stays 17 pt |

## This round: PhotoCraft checked live, "Lorem…", keys 18 pt

| Rule | In FontCraft |
|---|---|
| Live check in the person's PhotoCraft (scratch document, closed unsaved) | opened by macOS with FontCraft's channel variables, PhotoCraft listened on 127.0.0.1:7981 within a second; FontCraft's token authenticated; `type.fonts`, `type.setStyle` ("Ancizar" Helvetica → Georgia Regular) and `type.resolveMissingFonts` (nothing missing, text unchanged, no dialog) worked |
| Documents in the desktop app | the desktop PhotoCraft has no `session.list` ("unknown method… did you mean jobs.list or ui.menu.list?"): its open documents are in `ui.inspect` → `session.documents` (name, index; `active`). FontCraft asks that, then `session.list` (photocraft-cli serve) |
| PhotoCraft's font list | built the first time PhotoCraft needs it, then kept: a font turned on after that isn't in it (no command refreshes it: type.updateAllTextLayers, type.font.library, type.fonts, type.resolveMissingFonts, type.setStyle tried), so FontCraft asks to reopen PhotoCraft for it and keeps the font on for that |
| "Lorem…" when PhotoCraft reopens | PhotoCraft's own preference "Fill new type layers with placeholder" (`fillNewTypeLayersWithPlaceholder: true` in its preferences.json): clicking with its Type tool where there's no text makes a new type layer with Lorem ipsum. FontCraft never sends `type.pasteLoremIpsum` (no shortcut either); with FontCraft closed, PhotoCraft opened without any text edit |
| Keys for a character | 18 pt in the glyph preview and in Character Info (`metrics::KEYS_PT`) |

## This round: PressCraft

PressCraft (`app.presscraft`, 0.2.0, the person's layout, drawing and photo app; Rust, eframe/egui
0.37 with AccessKit) has no AppleScript, no control channel and no font message like Affinity's, and
it pastes plain text only (menus.rs: `Paste` → `arboard … get_text`). What FontCraft does with it
comes from its source (read only) and a check on a test copy of it (`PRESSCRAFT_TEST=seltext`, its
sample document, nothing saved):

| Rule | In FontCraft |
|---|---|
| Listed in Automation | "PressCraft", found by its bundle id (/Applications/PressCraft.app), with its icon; Windows `PressCraft.exe`, Linux `presscraft` |
| Its font list | read once when it opens (text.rs `scan_system` on a thread from `App::new`; no font-change listener, no command to read again): /System/Library/Fonts, /Library/Fonts and ~/Library/Fonts, four folders deep, so the fonts FontCraft keeps on in ~/Library/Fonts/FontCraft are in it when they were there before PressCraft opened |
| Does PressCraft have the family? | a file of that family is in those folders (or FontCraft's copy of it in ~/Library/Fonts/FontCraft) and was put there before PressCraft opened: its status-change time against PressCraft's launch date (`NSRunningApplication launchDate`, else the age of its process). Otherwise nothing is set, the font stays on and FontCraft says to reopen PressCraft (it stays 60 s after PressCraft quits, `REOPEN_GRACE_S`) |
| Font on the selection | through Accessibility, in PressCraft's own font field (studio2.rs `font_picker`): AccessKit gives macOS egui's widgets, and the field is the text field with the style menu (AXPopUpButton) right beside it on its row — its placeholder "Fuente" shows only while it is empty (accesskit_consumer exposes placeholders of empty fields only). FontCraft brings PressCraft to the front, sets AXFocused (PressCraft opens its list), sets AXValue to the family plus a space (PressCraft's search trims it; without it the name it shows would read as "nothing typed" and its list would start at A), presses Return (PressCraft chooses the first family of its list and `format_text` puts it on the selected text, the caret's paragraph or the selected text frames), and reads the field until it shows the family |
| The style | then the style menu beside the field is opened (AXPress) and the face's style chosen among its entries, found by asking what is at points below the menu (`AXUIElementCopyElementAtPosition`, one question each); its list is closed again (a click from Accessibility isn't a pointer click, which is what closes it). Checked: "Avenir Next Italic" → "Georgia Bold" on the sample's caption |
| Nothing or the wrong family | the field still showing the typed name: PressCraft has nothing of that name (Escape leaves the field as it was); another family that starts the same way chosen: taken back with ⌘Z. Either way FontCraft says to reopen PressCraft |
| No font field showing | no text tool and no text selected or being edited: "Select the text first"; a glyph isn't typed then (PressCraft takes typed letters as tool shortcuts) |
| Glyphs | typed at PressCraft's text cursor (keyboard events carrying the character), selected (⇧←) and put in the font with the field, then the cursor goes after it (→) |
| Its documents | PDFs carrying the editable document (a zip with `mimetype` = application/x-presscraft and document.json; fileio.rs `pdf_document_bytes`), older ones that zip alone (.presscraft). The window title names the one in front ("Cartel — PressCraft", "(editado)" while changed; app.rs) and its recent list (`~/Library/Application Support/PressCraft/settings.json`, "recent") has the full path |
| Missing Fonts | the "family" fields of document.json (text formats, character styles, tables) of the document in front; families not installed open the Missing Fonts window for PressCraft; families FontCraft has on that PressCraft didn't read when it opened: "reopen PressCraft" |
| Fonts going off | PressCraft keeps its documents as tabs of one window and its title names only the one in front, so which are open isn't known: fonts turned on for it stay until it quits (and 60 s more) |
| Pauses (estimated) | `PRESSCRAFT_STEP_MS` = 220 ms after each step; `PRESSCRAFT_PICK_WAIT_MS` = 1500 ms at most for the field to show the family; `PRESSCRAFT_TREE_TRIES` = 8 looks (AccessKit sends PressCraft's widgets a frame after the first question), every 250 ms; at most `PRESSCRAFT_TREE_NODES` = 6000 elements |
| Checks | `FontCraft --test-presscraft-field [family]` and `--test-presscraft-apply family [style]` against a running PressCraft |
