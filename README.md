# EmOrbit

An emoji wheel, always within reach.

EmOrbit is a lightweight Windows emoji picker being built around a radial dial. Select an emoji with a gesture, explore contextual variants, or pin the wheel while chatting.

## Project status

Experimental native prototype. The first implementation includes a radial wheel, contextual choices, local search, paging, pinning, a tray icon, and guarded Unicode insertion. It has a small catalog of 24 entries. Live compatibility, appearance, and resource use still require Windows testing. There is no stable release yet.

See [running and testing](docs/TESTING.md) for build instructions, controls, CI artifacts, and known limitations. The list below describes the full intended product, including features not implemented yet.

## Planned experience

* Hold a configurable activation shortcut, move toward an emoji, and release to insert.
* Pause over a selection to reveal an outer arc with related expressions, repeated versions, or supported skin tones.
* Click Search in the centre and type to replace the wheel contents with search results.
* Pin the picker to keep it visible while inserting multiple emojis.
* Preserve the original typing destination and avoid covering the writing area when its position can be detected.
* Browse the emoji collection through categories, result pages, and search.

## Technical direction

Rust with native Windows APIs, Direct2D for the interface, and DirectWrite for text and emoji rendering. Windows integration will use the `windows` crate. No browser engine, account, server, or AI is planned.

Memory usage and application compatibility have not been measured. Emoji appearance and glyph coverage depend on the Windows fonts available.

## Documentation

* [Interaction design](docs/DESIGN.md)
* [Architecture and validation](docs/ARCHITECTURE.md)
* [Roadmap](docs/ROADMAP.md)
* [Running and testing](docs/TESTING.md)
* [Contributing](CONTRIBUTING.md)

## License

[MIT](LICENSE). Future third party dependencies and emoji metadata retain their own licenses and notices.
