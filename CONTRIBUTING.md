# Contributing to EmOrbit

EmOrbit has an experimental native prototype. Start with the README and documents in docs before proposing implementation changes. The intended design exceeds the current implementation.

Focus first on the Windows integration proof. Prefer small, reviewable changes and explain their effect on typing focus, accessibility, and resource use. Preserve complete Unicode sequences rather than treating every emoji as a single character.

For bug reports, include the Windows version, target application, display scaling, reproduction steps, and expected versus observed behaviour. Do not include private messages or credentials.

For implementation changes, validate relevant pure logic and test affected Windows interactions. Distinguish measured behaviour from assumptions. Keep dependency and data licensing explicit.

See docs/TESTING.md for build instructions, controls, and manual acceptance checks. Run cargo fmt --all -- --check, cargo clippy --locked --all-targets -- -D warnings, and cargo test --locked before submitting a change. Windows native code must also compile for the Windows target; passing Linux tests alone does not check that module.
