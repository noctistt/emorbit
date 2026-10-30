# Contributing to EmOrbit

EmOrbit is currently in the design stage. Start with the README and documents in docs before proposing implementation changes.

Focus first on the Windows integration proof. Prefer small, reviewable changes and explain their effect on typing focus, accessibility, and resource use. Preserve complete Unicode sequences rather than treating every emoji as a single character.

For bug reports, include the Windows version, target application, display scaling, reproduction steps, and expected versus observed behaviour. Do not include private messages or credentials.

For implementation changes, validate relevant pure logic and test affected Windows interactions. Distinguish measured behaviour from assumptions. Keep dependency and data licensing explicit.

A contributor build guide will be added with the initial application scaffold.
