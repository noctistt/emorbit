# Interaction design

Status: proposed behaviour requiring Windows prototypes and user testing.

## Quick selection

A configurable hold shortcut opens the dial near the pointer. Approximately eight stable primary positions show favourites or categories. Movement highlights a segment. Releasing the activation gesture commits a valid highlighted selection. Returning to the central cancel area or pressing Escape cancels without insertion. Exact bindings remain undecided; overriding Windows emoji shortcuts is deferred.

A brief dwell opens a contextual outer arc. Movement through a segment should not immediately open another menu. Use forgiving pointer corridors and stable geometry so the submenu remains reachable.

## Contextual choices

Expression choices can expose related expressions and explicit single, double, or triple repetition. Supported person and hand sequences can expose skin tones. Unsupported combinations must never be synthesized. More complex sequences require valid Unicode data.

Preview the exact output before committing. Store an optional preferred tone locally, but do not silently alter already saved favourites. Keep favourite positions stable; recent items have a separate area or category.

## Search

Clicking the central search button enters a persistent search state: the user can release the activation shortcut and type. Search replaces the primary wheel results. Result pages are navigable with the mouse wheel and keyboard, with a visible page indicator. A clear action restores the home dial. Enter inserts the highlighted result. Escape exits search before dismissing the picker.

Search temporarily requires keyboard focus. Retain the external destination separately so selecting a search result does not target the picker itself.

## Pinned mode

The pin button keeps the picker visible after insertion. Drag a centre handle to reposition it without accidentally selecting an emoji. Remember its position, but clamp it to an available monitor after display changes.

Emoji buttons should avoid activating the window where practical. Track the most recently focused eligible external text destination while pinned. Never send input to an invalid, destroyed, or unexpectedly changed target. If the destination cannot be established, request that the user focus a text field.

Search explicitly activates an input surface. Finish selection by returning to the captured destination and keep the pinned dial visible. Provide accessible alternatives for every gesture.

## Placement and appearance

Modern light and dark themes, rounded segments, clear selection states, short animations, and optional transparency. Provide opaque and reduced motion modes. Respect scaling, monitor work areas, and high contrast preferences.

Choose above or below the caret when its rectangle is available and the layout fits. Otherwise use a predictable pointer or user selected fallback. Positioning cannot guarantee that all underlying content stays unobscured.

## Accessibility

Keyboard navigation, readable labels, screen reader semantics, sufficiently large targets, visible focus, and alternatives to hold gestures are requirements. Custom drawing must not make the controls inaccessible.
