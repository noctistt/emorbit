# Running the native prototype

This is an experimental Windows integration build, not a finished release. It has a manually curated catalog of 29 entries. The full catalog, saved preferences, configurable shortcuts, screen reader support, and polished exploration controls remain future work.

## Build on Windows

Install stable Rust with the MSVC toolchain and the Visual Studio C++ build tools, including a Windows SDK. From the repository directory:

```powershell
cargo test --locked
cargo build --locked --release
.\target\release\emorbit.exe
```

The application opens its wheel on launch and also provides a standard placeholder tray icon. It does not install itself, request administrator access, start with Windows, access the network, or use the clipboard. Exit through the tray icon's context menu.

## Download a CI build

For normal use, download **EmOrbit-Setup.exe** directly from the latest preview on the repository Releases page. Run it once and use the Start menu shortcut. There is an optional desktop shortcut, and uninstall is available in Windows Settings. Setup installs into your local Programs folder without requesting administrator access. Running a newer setup updates that same installation. Setup asks a running EmOrbit to exit gracefully before replacement; if that fails, close it from the tray and retry.

Both the app and installer are unsigned. There is no automatic updater or stable release yet. Preview releases include a SHA256 checksum file.

Open the repository Actions tab, choose a successful Windows prototype run for the desired commit, and download its EmOrbit-Windows-x64 artifact while signed in to GitHub. Extract the ZIP and run emorbit.exe from the artifact. The build is unsigned; Windows may show a reputation prompt. Confirm the repository and commit before running it.

The ZIP artifact remains available for portable testing. It is not required when using the setup executable.

## Installer development

The Inno Setup source is packaging/emorbit.iss. After cargo build --release, compile it with ISCC.exe packaging/emorbit.iss from the repository root, passing /DAppVersion to override the version when required. GitHub Actions extracts the version from Cargo.toml automatically. The stable AppId and installation directory must be preserved across updates. The graceful close code uses the native window class and exit message; keep those synchronized with platform.rs.

The Windows workflow compiles setup and runs packaging/test-installer.ps1 in an isolated runner profile. It checks silent installation, executable integrity, a repeated installation, Start menu registration, and uninstall cleanup. It does not validate interactive wizard appearance or live upgrades with a running application. Successful push builds publish a uniquely tagged preview release with the installer as a directly downloadable asset. Pull request builds never publish releases.

## Controls

* Focus a text field in Notepad first.
* Tap Ctrl + Alt + Space and release the keys. The picker stays open. Releasing shortcut keys does not select an emoji.
* Press and hold the primary mouse button in the centre or over an emoji, move onto the desired emoji, then release the mouse button to insert. A normal click also inserts.
* Hover an emoji with variants to open its contextual arc immediately. There is no dwell delay. You can keep the mouse button held while exploring the arc; release over the desired variant to insert it.
* Suggestions now fan outward beside their parent emoji. Move back to the centre to dismiss the fan. While a fan is open, paging arrows and footer text are hidden to leave space for lower suggestions; use the mouse wheel to change pages.
* Repeated choices show a small ×2 or ×3 badge instead of overflowing strings. The header previews the actual output on hover. Highlights fade and gently grow, and suggestions expand into position. Windows' client area animation preference is read at startup and on settings changes.
* Move back to the centre after pressing an emoji, or release outside the targets, to cancel. The unpinned picker closes on cancellation; the pinned picker stays open. A stationary centre click opens Search instead.
* A drag passing over Pin, Close, or a page arrow does not activate it. Click those controls normally. The primary mouse button respects Windows' swapped button setting.
* Click the magnifying glass in the centre to activate its text field. Type a query; results replace the wheel. Click a result or press Enter to insert the first result on the current page. The search box is limited to 255 UTF-16 units.
* Scroll over the wheel, or click the arrows, to change result pages.
* Click Pin to keep the wheel open. Click emojis while the external text field remains focused. Click Unpin to return to closing after selection.
* Drag the header to move the popup. It does not yet remember its position between openings.
* Escape exits search first, then dismisses the picker. Escape is temporarily registered as a global hotkey while the picker is visible. If another app owns that hotkey, use the close button.
* The main shortcut toggles an already visible picker closed. A conflicting shortcut prevents startup and displays an error.
* Left click the tray icon to open a persistent popup; focus a text field before clicking an emoji. Right click the tray icon to exit.

## Initial acceptance checks

Use a disposable text document. Record app and Windows versions and actual observations. No interactive Windows checks have yet been completed in the development container.

| Check | Expected result | Status |
| --- | --- | --- |
| Quick selection in Notepad | Complete emoji inserted once; popup closes | Pending Windows test |
| Shortcut tap and key release | Picker stays open; no emoji inserted | Pending Windows test |
| Mouse hold from centre to emoji | Mouse release inserts the final target exactly once | Pending Windows test |
| Mouse hold from emoji to variant | Mouse release inserts that variant, not the initial emoji | Pending Windows test |
| Mouse release outside the popup | Cancels without inserting the last highlighted emoji | Pending Windows test |
| Drag across toolbar controls | Does not accidentally pin, close, or change pages | Pending Windows test |
| Hover each of the eight home emojis | Suggestions appear near that emoji, including bottom and side positions | Pending Windows test |
| Quick movement between emojis and variants | Smooth highlights with clickable targets matching their visible positions | Pending Windows test |
| Disable Windows animations | Transitions snap to their endpoints without repeated animation frames | Pending Windows test |
| Transparent background | Underlying content is visible between solid emoji buttons; gaps pass clicks through | Pending Windows test |
| Immediate suggestions | Entering an emoji creates its fan without waiting for a dwell timer | Pending Windows test |
| Search after transparency change | Search popup accepts text and restores external typing focus after selection | Pending Windows test |
| Pinned wheel and continued typing | Emoji clicks leave the external field usable | Pending Windows test |
| Pinned target switch | Selecting after switching windows inserts into the current focused destination | Pending Windows test |
| Search then selection | Focus returns and the selected sequence inserts | Pending Windows test |
| Change focus during search | First selection cancels rather than forcing input into an old destination | Pending Windows test |
| Escape and empty space release | No text inserted | Pending Windows test |
| Tones, hearts, flag, joined emoji | Complete sequences arrive without broken surrogate pairs | Pending Windows test |
| Browser input and complex editor | Record compatibility; do not assume support | Pending Windows test |
| Elevated application | May reject injection; no elevation workaround | Pending Windows test |
| Negative monitor coordinates and mixed DPI | Controls remain aligned and reachable | Pending Windows test |
| Exit and restart | Tray icon and shortcut are released | Pending Windows test |

## Known limitations

The popup uses smooth alpha transparency with solid controls and pointer based placement, not caret avoidance. A native search edit in a small owned popup is intentionally basic. Emoji rendering depends on system font coverage. Complex or protected editors may not accept Unicode SendInput. Transparent gaps pass mouse input to the underlying application; hold gestures are tracked by the existing short gesture timer while crossing these gaps.

The destination check compares foreground window and focused control handles. It cannot distinguish every cursor or tab change inside a custom editor that reuses one native control. It does not read the target's document text. Input stops if the captured handles no longer match or modifiers remain held for two seconds. A failed or partial injection is reported without retrying the string.

Favorites are saved locally; pinned state, recents, positions, and preferred skin tones are not persisted yet. Keyboard navigation beyond search Enter and Escape and accessibility providers are not implemented. The standard Windows emoji shortcuts are left untouched. The app does not automatically retry tray registration after an Explorer restart yet.

## Resource measurements

Use a release build, not cargo run in debug mode. Record private bytes, working set, and CPU while hidden, open, searching, and pinned, then repeat opening and closing. No memory or latency benchmarks have been established. The hidden app uses a blocking message loop. A 16 millisecond gesture timer runs only during an active mouse press that began in the picker or pending insertion. This checks pointer position and primary button release even outside the popup without a global mouse hook or focus stealing capture. A separate animation timer runs only while finite highlight or expansion transitions are active and stops at rest. There is no dwell timer. Windows native tests check the configured color key, empty background pixels, solid controls, vector search icon pixels, immediate context creation, and intensity selection.

### Floating edges and normal reaction suggestions

Version 0.1.8 uses premultiplied per pixel alpha for smooth edges on light backgrounds, with explicit physical position and surface dimensions on every UpdateLayeredWindow presentation. The software Direct2D target clears to transparent black. Layered styles are reset before switching between alpha and color key APIs. Reentrant size notifications during presentation are ignored to avoid a render/resize loop.

The default floating mode uses smooth alpha. **Smooth edges** in the tray menu switches to or from the color key compatibility renderer. **Floating background** toggles a full solid background. An alpha API failure falls back to color key drawing. A one shot check after the first frame compares one screen pixel at the center of our own solid search button with the expected color; a mismatch also restores compatibility mode. This check does not capture or save screenshots and is skipped if the screen pixel cannot be read or native search is active. A tolerance accommodates display color differences. Compatibility mode can retain rough edges, so live smooth appearance still needs confirmation on the affected display.

There is no Intensity button or separate mode. Normal nearby fans retain up to four standard choices and add two named stronger reactions for supported emotions. Sadder inserts 😢💔 and Saddest inserts 😭💔🥀; Angrier inserts 😡💢 and Angriest inserts 🤬💢🔥. Other named reactions cover laughter, love, happiness, celebration, and fire. Named choices use readable text inside the existing bubbles rather than clipped emoji sequences. Hover shows the complete output in a compact preview pill. Release inserts that exact sequence. Hand suggestions keep all six skin tone choices.

The toolbar is one compact rounded surface containing the title, pin control, and close control. The always visible instruction/status strip is removed; a short preview appears only while a suggestion is hovered. Paging uses one compact surface.

Native tests verify transparent gaps, fractional alpha around toolbar corners and glows, solid search/emoji controls, search icon pixels, normal reaction selection without a mode switch, and opening/reopening across smooth, compatible, and solid presentations. Pure tests verify automatic reaction mixing, named output, preserved skin tones, geometry, and UTF16 sequences.

Startup and first frame diagnostics remain in `%LOCALAPPDATA%\EmOrbit\emorbit.log`, without searches or inserted text. The display check records only the observed color of the app's own center button.

### Favorites and edge paging (0.1.9)

★ toggles a separate favorites dial. + opens a native edit where you can paste an emoji or combination, including ones outside the 29 entry catalog, and press Enter to save it. Nothing is inserted into the external editor when saving. Right click an item or suggestion for Save to favorites; right click a saved item to remove it. Known emoji favorites retain their normal variants. Duplicate sequences are ignored. Favorites are stored in `%LOCALAPPDATA%\EmOrbit\favorites.txt` using an atomic replacement; a save error restores the prior in memory store and displays an error. The store supports 128 entries, each at most 128 UTF16 units, and rejects empty, control character, and ASCII only entries. It preserves Unicode sequences verbatim rather than enforcing the full Unicode emoji standard. Startup loads at most 64 KiB. Mode choice is session only.

The toolbar is lowered to immediately above the main dial. It yields its space while suggestions expand so it cannot cover an upper fan; return to the center to dismiss the fan and access it. Left and right arrow buttons sit at the sides of the dial and yield to an active fan. A compact page count remains below.

During a held selection gesture, entering the outer left or right edge turns one page. Each excursion turns only once; return through the central band before another turn. The same gesture can then select an item on the new page. Releasing at an edge turns no further pages, inserts nothing, and leaves the picker open. Clicking page buttons still turns exactly once. The gesture uses the existing active press timer and adds no idle polling.

Tests cover favorite save/load/update/removal and Unicode preservation, arbitrary favorite selection, known favorite variants, side arrow hit regions, and one page per edge excursion. Windows still needs manual checks for favorite input focus, context menu dismissal, and held edge gestures on the affected display.
