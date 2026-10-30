# Interaction design

Status: proposed behaviour requiring Windows prototypes and user testing.

## Quick selection

A shortcut tap opens the dial near the pointer and leaves it open after the keys are released. Press and hold the primary mouse button in the centre or over an emoji, move to a selection, then release the mouse button to insert. A normal emoji click also works. Approximately eight stable primary positions show favourites or categories. Returning from an emoji to the centre or releasing outside the targets cancels without insertion. A stationary centre click opens Search. Pressing Escape dismisses the picker. The prototype shortcut is Ctrl + Alt + Space; configuration and overriding Windows emoji shortcuts are deferred.

Releasing a gesture over Pin, Close, or a page arrow must not activate that control unless the press began on the same control. Releasing the shortcut keys never commits a mouse gesture. Only a press originating in the picker starts gesture tracking; ordinary clicks elsewhere do not select anything. Respect swapped primary mouse buttons.

Hovering a primary emoji opens its contextual arc immediately, with no dwell timer. A brief expansion animation gives visual continuity without delaying creation of the suggestions. Preserve the parent menu while traversing transparent gaps towards its suggestions. Switching to another primary emoji replaces the fan immediately.

Suggestions form a local outward fan anchored to the parent emoji, rather than a fixed row above the wheel. The fan rotates with its parent and fits inside the existing popup. Dim other primary choices while it is open. Temporarily hide page arrows and footer text where a lower fan needs that space; the header shows the exact selected output and scrolling still changes pages. Returning to the centre dismisses the fan.

Hover highlights transition through colour and slight size changes over about 140 milliseconds; suggestion fans expand over about 180 milliseconds. Rendering and hit testing share the animated positions. Use finite transitions, stop the animation timer at rest, and respect Windows' client area animation preference. Repeated output uses a single emoji with a compact ×2 or ×3 badge, with the complete output previewed in the header.

## Contextual choices

Expression choices can expose related expressions and explicit single, double, or triple repetition. Supported person and hand sequences can expose skin tones. Unsupported combinations must never be synthesized. More complex sequences require valid Unicode data.

Preview the exact output before committing. Store an optional preferred tone locally, but do not silently alter already saved favourites. Keep favourite positions stable; recent items have a separate area or category.

## Search

Clicking the central search button enters a persistent search state and activates the search input. Search replaces the primary wheel results. Result pages are navigable with the mouse wheel and keyboard, with a visible page indicator. A clear action restores the home dial. Enter inserts the highlighted result. Escape exits search before dismissing the picker. The prototype currently inserts the first result on Enter; full keyboard navigation remains planned.

Search temporarily requires keyboard focus. Retain the external destination separately so selecting a search result does not target the picker itself.

## Pinned mode

The pin button keeps the picker visible after insertion. Drag a centre handle to reposition it without accidentally selecting an emoji. Remember its position, but clamp it to an available monitor after display changes.

Emoji buttons should avoid activating the window where practical. Track the most recently focused eligible external text destination while pinned. Never send input to an invalid, destroyed, or unexpectedly changed target. If the destination cannot be established, request that the user focus a text field.

Search explicitly activates an input surface. Finish selection by returning to the captured destination and keep the pinned dial visible. Provide accessible alternatives for every gesture.

## Placement and appearance

The prototype uses a transparent background with solid emoji buttons and compact dark backing pills for its header and controls. A magnifying glass drawn with vector geometry replaces the centre Search label. Light themes, an opaque fallback mode, and full high contrast adaptation remain planned. Respect scaling, monitor work areas, and Windows' animation preference.

Choose above or below the caret when its rectangle is available and the layout fits. Otherwise use a predictable pointer or user selected fallback. Positioning cannot guarantee that all underlying content stays unobscured.

## Accessibility

Keyboard navigation, readable labels, screen reader semantics, sufficiently large targets, visible focus, and alternatives to hold gestures are requirements. Custom drawing must not make the controls inaccessible.
