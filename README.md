# EmOrbit

An emoji wheel, always within reach.

EmOrbit is a lightweight Windows emoji picker being built around a radial dial. Select an emoji with a gesture, explore contextual variants, or pin the wheel while chatting.

## Project status

Experimental native prototype. The implementation includes a radial wheel, parent anchored suggestion fans, animated highlights, local search, paging, pinning, a tray icon, and guarded Unicode insertion. It has a small catalog of 24 entries. Early user testing confirmed emoji insertion and mouse release selection; broader compatibility, visual polish, and resource use still require Windows testing. There is no stable release yet.

See [running and testing](docs/TESTING.md) for build instructions, controls, CI artifacts, and known limitations. The list below describes the full intended product, including features not implemented yet.

## Planned experience

* Tap an activation shortcut to open, then hold the primary mouse button, move toward an emoji, and release the mouse button to insert.
* Pause over a selection to reveal an outer arc with related expressions, repeated versions, or supported skin tones.
* Click Search in the centre and type to replace the wheel contents with search results.
* Pin the picker to keep it visible while inserting multiple emojis.
* Preserve the original typing destination and avoid covering the writing area when its position can be detected.
* Browse the emoji collection through categories, result pages, and search.

## Technical direction

Rust with native Windows APIs, Direct2D for the interface, and DirectWrite for text and emoji rendering. Windows integration will use the `windows` crate. No browser engine, account, server, or AI is planned.

Memory usage and application compatibility have not been measured. Emoji appearance and glyph coverage depend on the Windows fonts available.

The current shortcut is Ctrl + Alt + Space. Release those keys after opening; selection uses the mouse button. You can also click an emoji normally. Configurable shortcuts remain planned.

## Documentation

* [Interaction design](docs/DESIGN.md)
* [Architecture and validation](docs/ARCHITECTURE.md)
* [Roadmap](docs/ROADMAP.md)
* [Running and testing](docs/TESTING.md)
* [Contributing](CONTRIBUTING.md)

## License

[MIT](LICENSE). Future third party dependencies and emoji metadata retain their own licenses and notices.
