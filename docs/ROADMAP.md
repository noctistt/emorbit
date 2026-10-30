# Roadmap

The first native implementation now exists. Implementation is not the same as verified user behaviour: interactive Windows acceptance checks remain pending in TESTING.md. Later milestones describe the full intended product.

## 1. Windows integration proof

* Implemented: Rust scaffold, native renderer, small catalog, wheel geometry, search, contextual variants, pinning, tray controls, and guarded input path.
* Implemented: fixed Ctrl + Alt + Space shortcut and documented Windows CI build. Shortcut configuration remains future work.
* Verified in the development container: pure logic tests and Windows target type checking; see TESTING.md for current validation boundaries.
* Pending: interactive insertion, pinned selection, search focus transitions, mixed DPI behaviour, and appearance on Windows.
* Pending: compatibility and resource measurements.

## 2. Radial interaction

* Draw a compact dial with stable positions and pointer hit testing.
* Implement hold, move, release, central cancel, and Escape.
* Add contextual arcs with dwell timing and safe navigation.
* Support screen boundaries and display scaling.

## 3. Catalog and search

* Add a licensed emoji catalog with valid sequences and search metadata.
* Search directly in the centre and display results in the wheel.
* Provide categories, result pages, keyboard selection, and an exploration view.
* Support valid skin tones and explicit repetition choices.

## 4. Daily use

* Pin, unpin, drag, and restore placement.
* Save favourites, recent selections, and preferences.
* Add themes, reduced motion, accessible controls, and compatibility fallbacks.
* Verify behaviour across a documented application matrix.

## 5. Distribution

* Automate Windows builds and appropriate checks.
* Publish installation instructions and signed releases when signing is available.
* Include dependency and dataset notices.
* Publish measured resource use and known limitations.

Do not mark a feature complete until its behaviour has been verified. The first milestone resolves the primary product risk: reliable insertion and focus management.
