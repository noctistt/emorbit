# Running the native prototype

This is an experimental Windows integration build, not a finished release. It has a manually curated catalog of 24 entries. The full catalog, saved preferences, configurable shortcuts, screen reader support, and polished exploration controls remain future work.

## Build on Windows

Install stable Rust with the MSVC toolchain and the Visual Studio C++ build tools, including a Windows SDK. From the repository directory:

```powershell
cargo test --locked
cargo build --locked --release
.\target\release\emorbit.exe
```

The application starts hidden with a standard placeholder tray icon. It does not install itself, request administrator access, start with Windows, access the network, or use the clipboard. Exit through the tray icon's context menu.

## Download a CI build

Open the repository Actions tab, choose a successful Windows prototype run for the desired commit, and download its EmOrbit-Windows-x64 artifact while signed in to GitHub. Extract the ZIP and run emorbit.exe from the artifact. The build is unsigned; Windows may show a reputation prompt. Confirm the repository and commit before running it.

There is no published stable release or installer yet.

## Controls

* Focus a text field in Notepad first.
* Tap Ctrl + Alt + Space and release the keys. The picker stays open. Releasing shortcut keys does not select an emoji.
* Press and hold the primary mouse button in the centre or over an emoji, move onto the desired emoji, then release the mouse button to insert. A normal click also inserts.
* Hover for 350 milliseconds over an emoji with variants to open its contextual arc. You can keep the mouse button held while exploring the arc; release over the desired variant to insert it.
* Move back to the centre after pressing an emoji, or release outside the targets, to cancel. The unpinned picker closes on cancellation; the pinned picker stays open. A stationary centre click opens Search instead.
* A drag passing over Pin, Close, or a page arrow does not activate it. Click those controls normally. The primary mouse button respects Windows' swapped button setting.
* Click Search in the centre to latch the popup open and activate its text field. Type a query; results replace the wheel. Click a result or press Enter to insert the first result on the current page. The search box is limited to 255 UTF-16 units.
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

The popup currently uses a rectangular opaque dark surface and pointer based placement, not caret avoidance. A native search edit control is intentionally basic. Emoji rendering depends on system font coverage. Complex or protected editors may not accept Unicode SendInput.

The destination check compares foreground window and focused control handles. It cannot distinguish every cursor or tab change inside a custom editor that reuses one native control. It does not read the target's document text. Input stops if the captured handles no longer match or modifiers remain held for two seconds. A failed or partial injection is reported without retrying the string.

Pinned state, favorites, recents, positions, and preferred skin tones are not persisted yet. Keyboard navigation beyond search Enter and Escape and accessibility providers are not implemented. The standard Windows emoji shortcuts are left untouched. The app does not automatically retry tray registration after an Explorer restart yet.

## Resource measurements

Use a release build, not cargo run in debug mode. Record private bytes, working set, and CPU while hidden, open, searching, and pinned, then repeat opening and closing. No memory or latency benchmarks have been established. The hidden app uses a blocking message loop; a 16 millisecond timer runs only during an active mouse press that began in the picker or pending insertion. This checks pointer position and primary button release even outside the popup without a global mouse hook or focus stealing capture. The hover timer is cancelled after it fires.
