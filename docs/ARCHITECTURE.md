# Architecture and validation

## Proposed stack

* Rust for application logic.
* Native Win32 windows and input integration through the windows crate.
* Direct2D for radial shapes and animation.
* DirectWrite for text and colour emoji.
* Local preference files for favourites, recent entries, skin tone, placement, and shortcut configuration.
* A locally searchable Unicode emoji catalog with reviewed licensing and provenance.

Dependency versions, minimum Windows version, packaging, and installation method are not yet selected. No Rust application or build commands exist in the repository yet.

## Suggested boundaries

Keep geometry and hit testing, interaction state, emoji sequences and search, preferences, Windows focus and insertion, and rendering separate. Pure logic should be testable without Windows.

Represent states explicitly: hidden, quick selection, contextual selection, persistent search, and pinned. Prevent key release after entering search from accidentally committing a gesture.

## First technical proof

Prove a small popup can insert a Unicode emoji into an external field and remain visible without stealing typing focus when pinned. Validate search focus transitions before investing in visual polish.

Capture destination identity before activation. Use available caret and accessibility information for placement; fall back when unavailable. Monitor focus events rather than continuously polling the desktop. Keep callbacks short and perform potentially slow accessibility queries outside latency sensitive input handlers.

Evaluate Unicode input through SendInput and measure compatibility. Windows integrity boundaries can block input into elevated applications. Do not claim universal insertion support or run elevated by default. Clipboard paste, if ever added as a fallback, requires careful preservation and race handling; do not silently overwrite the clipboard.

## Resource use

Wait for input when hidden. Render in response to changes. Schedule animation only while transitions are active. Avoid creating a visual object or bitmap for every catalog entry. Load only necessary resources and bound caches.

Measure release builds while hidden, open, searching, and pinned. Report private bytes and working set separately; measure process CPU, opening latency, search responsiveness, and resource stability after repeated use. No RAM or latency claims are established yet.

## Compatibility validation

Test Notepad, browser text fields, a complex browser editor, Word if available, a chat client, and Windows Terminal. Record operating system, app version, insertion method, focus behaviour, and failures.

Test supplementary Unicode characters, variation selectors, skin tones, flags, and joined sequences. Test multiple monitors with mixed scaling, screen edges, disconnected monitors, keyboard layouts, search cancellation, lost destinations, and elevated targets.

## Privacy and licensing

No telemetry, networking, accounts, or content logging is planned. Do not collect typed messages or persist external field contents. Store only intentional application preferences and recent emoji selections. Review data licenses before importing Unicode names or keyword datasets. Do not redistribute Windows fonts.
