# Triwarden-inspired layout

Reference: local `triwarden` checkout at revision
`2271fdfa0119d3f03e7aa18c3f7177d71c43fcf7`, read-only. Inspected sources are
`App/Sources/VaultView.swift`, `App/Sources/SidebarAccordion.swift`,
`App/Sources/PaneStrip.swift` and `App/Sources/WindowBackdrop.swift`. No Swift
source was copied; the application remains GPL-3.0-only and Plico stays
Apache-2.0.

## Shell

The vault is three floating panes over a pastel wash instead of flush,
opaque columns:

| Part | Value |
| --- | --- |
| Window inset | 8 logical px, with an 8 px gap between panes |
| Header | 48 px, full width; controls and the centered command-palette trigger float over the wash |
| Pane top | Every pane starts at the same y below the header |
| Navigation | 260 px (220 px below 1120 px viewport); 18 px radius |
| Item list | 320 px (300 px in the compact pair); 18 px radius; sidebar counts exclude trashed and archived items; reverse and type sorting |
| Cards | 18 px radius, 24 px item hero |
| Compact pair | Below 900 px the sidebar becomes a separate pane; list and detail share the window |
| Single pane | Below 640 px only the list or detail shows, with a Back control |
| List rows | 56 px, 13 px radius, 38 px brand tile, A–Z headers |
| Detail | 18 px card radius, 24 px hero padding; cards run to the pane edge |
| Sidebar rows | 28 px, 8 px radius, 160 ms selection/hover transition |
| Footer | Floating 32 px white capsule with Sync and Lock at the lower right |

The panes share one top edge and one 18 px radius, so their corners line up.
The header spans the whole window and carries the window controls, the pane
back control, the title and the actions. When an item detail is visible the
item actions (edit, favorite, duplicate, archive, folder, trash) replace the
list's global controls in the header, as in Triwarden, so the detail begins
with its hero card instead of an actions row. The sidebar no longer reserves
a blank strip for the traffic lights. At narrow widths the sidebar instead
opens as a full-width pane. Below 900 px the sidebar leaves the row and the
list and detail share the window; below 640 px one pane shows at a time and a
Back control returns to the list. Hidden content is not declared, so it leaves
hit testing and the accessibility tree immediately.

Folders are a tree: each folder name is split on `/`, the segments become
nested rows with 14 px indentation, and a parent with children shows a
chevron. Selecting a row filters by its normalized path, so a parent includes
every descendant; the count beside a row is the subtree count. Collapsed
parents keep their state in the application model across rebuilds.

Folders participate in the sidebar's outer scroll even when the tree is long;
the Folders manager list is capped at 360 px and scrolls internally. The
Items, Folders and Utilities headings fold their rows. The container has
an explicit height (rows are all 28 px with a 4 px gap), so the fold animates
through Plico's style transition: the height and content opacity run over
220 ms, the rows are clipped, and the closed group is disabled so it cannot
take focus. The chevron changes between the right and down shapes.

## Notifications

The header's bell sits with the global actions and carries an unread count.
Its anchored sheet lists background/error diagnostics and device signing
requests. Opening the sheet marks error and session-expired items read while
pending device requests stay unread until resolved; Review opens the SSH
approval sheet, Sign in locks the vault and returns to the sign-in form, and
Copy copies an error's detail. Escape, the scrim, or the close button dismiss
the sheet. A 401 from background sync becomes a Session expired notification
instead of a raw inline error; it does not force a lock on its own.

## Command palette

The header's centered `Search or run a command` pill (⌘K / Ctrl+K, or ⌘F /
Ctrl+F, also from the search icon on narrow windows) opens a modal palette with a focused search
field, the six global commands and up to eight matching vault items. ⌘, / Ctrl+, toggles Settings, ⌘G / Ctrl+G opens the
Generator, ⌘N / Ctrl+N starts a new item and ⌘L / Ctrl+L locks an unlocked
vault. Outside a text field, ↑/↓ move the list selection and reveal the row. Neither shortcut leaks into a focused text field: combinations
are handled only after controls and IME decline them. Choosing an item returns to All Items, selects it, reveals its list row and
closes the palette; choosing a command runs it directly (New item, Generator, Settings,
Sync, Lock, Accounts). Up/Down and PageUp/PageDown move the highlighted row,
which is scrolled into view while the search field stays pinned, Enter runs
it, and Escape closes the palette. Typing resets the highlight to
the first match. Narrow windows show the same palette behind a search
icon because the centered pill no longer fits.

## Editor

The editor is a page header (New item / Edit item, Cancel) followed by cards:
Item (type, name, favorite, folder), Fields (kind-specific inputs and Generate),
Notes, and Custom fields (add/remove rows). Save item sits at the end of the
form; the whole page scrolls as one column instead of a single tall card.

## Generator

The generator keeps Triwarden's hierarchy: a header card with the mode switch,
a result card that shows the generated value at 24 px with Generate and Copy,
then Options and History cards side by side. Opening the page or switching
mode generates a value immediately, matching the reference's regenerate-on-
change behaviour; recent values remain in the in-memory history card.

## Empty and filtered lists

When a sidebar filter is active the list shows a compact `Favorites ×`-style
chip above the rows; clicking it returns to All Items, which matters on compact
windows where the sidebar is a separate pane. An empty result surface offers
`Clear search` and `Show all items` actions instead of a dead end. A genuinely
empty vault centers its own surface with an `Add item` action that opens the
editor. The SSH agent sheet exposes a copy control for the socket path.

## List sorting

The list toolbar toggles descending order and switches between title sorting
and type sorting. Type mode groups rows under Logins, Cards, Identities,
Secure notes and SSH keys headings, and the same grouping drives selection
reveal offsets.

## Accounts

The account switcher lists saved accounts with an avatar, server and a
`Current` badge for the active account; the empty state explains that saved
accounts stay encrypted on this device. Forget and Add account remain one
click from the same sheet.

## Login

Wide windows split the sign-in screen like Triwarden's: a white brand pane on
the left (mark, tagline and three capability lines) and a pale form pane on the
right with the centered login card. Below 900 px the brand pane is dropped and
the existing centered card is shown over the wash. The form keeps the
email → password → verification steps, with the Server row inside the card
above Email; expanding it opens the region / URL sheet directly below.

## Surfaces

Panels and cards are solid white with a hairline border, an 18 px radius and
one soft shadow; fields use the theme's control fill. The window wash is one full-window WGSL pass
(`assets/backdrop.wgsl`) approximating Triwarden's `WindowBackdrop`: pink,
peach, mint and lavender radial glows over a light lavender base. It is
visible in the 8 px window inset, behind the floating header, and across the
login screen; it does not tint the text inside the panes.

The earlier glass material pass was removed. It washed out text and fields on
pale cards and cost backdrop-filter work on every pane. The layout no longer
depends on a shader: with `Reduce Transparency` the wash is skipped and the
solid surfaces remain unchanged. `Reduce Motion` snaps the sidebar width/gap
and row transitions.

Native text fields follow MyGO's recipe: the theme control fill, a 1 px border
in the theme border color, a 6 px radius, and the accent border while a
descendant editor is focused. The native editor text, selection and caret sit
in that solid shell; placeholder hints come from the application.

The live detail also shows the item's folder as a `Work › Backend` breadcrumb
and gives each website and the notes their own copy control. Custom fields render in their own card: text values can be copied, hidden
values stay masked, and booleans read Yes/No without a copy action. One-time
codes have Generate and Copy actions beside the draining ring. Card numbers are masked to the last four digits and can be revealed or hidden in place; expiry
and security-code availability sit alongside. Identity details include
company, address, city, state, postal code and country, while SSN, passport and
license numbers report availability instead of printing the value. Password history entries show a relative last-used time
instead of raw Unix seconds. SSH key items show the OpenSSH public key and SHA256 fingerprint with copy controls,
and report the private key as available rather than printing it. Security settings
groups its controls in the same bordered card as Appearance.

Settings opens as an instant modal sheet with its own section sidebar
(Appearance, Security, SSH agent), so it never replaces the vault columns and
never animates a full page of controls. Its height follows the current pane
instead of leaving a blank lower half.

## Verification

- `cargo test --locked --workspace` covers layout bounds, semantics, input
  focus, native preferences and the existing feature suite.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`.
- `python3 scripts/run-recipes.py --suite recipes/suite.ts` renders the vault,
  search, settings, editor, login, transfer and SSH surfaces with synthetic
  data.

The reduced-transparency test asserts zero shader declarations; the ordinary
render test asserts that the pastel wash is present. Screenshots
and measurements stay under `reports/` and are not committed.
