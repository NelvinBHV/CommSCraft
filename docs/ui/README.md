# Desktop interface

> **Last reviewed:** 2026-11-16 · **Last updated:** 2026-11-16 · **Change:** minor (desktop client: transfer, Kerberos)

The Linux desktop client `commscraft` (`apps/commscraft`) puts an egui interface on top of the
session engine. The interface lives in its own crate, `commscraft-ui` (`crates/ui`): theme, view
model, views and demo fixtures, with no protocol, network or file code. The protocol core never
depends on it.

```text
apps/commscraft        window (eframe, OpenGL, Wayland/X11), system fonts, brand icon, adapters
  └── crates/ui        CommsUi: views + theme + AppState (reducer) + UiIntent; demo fixtures
        ⇅ AppEvent / UiIntent (typed; never raw SIP)
      crates/session   the sans-I/O engine (unchanged)
```

## Phases

| Phase | Content | State |
|---|---|---|
| 1 | Shell and the three screens with demo data: chats (list, conversation, composer, group details), contacts and presence, calls (active call, controls, history), meetings, settings with the capability overview; keyboard, focus, scaling, high contrast, German and English | done (tested-offline) |
| 2 | The session adapter: sign-in, contacts and presence, 1:1 and group chat, delivery, typing, audio calls, meetings, invitations | done (tested-offline; `tested-in-lab`: sign-in, contacts and presence, 1:1 and group chat both ways, outgoing calls with ringing, connected, hold, resume, hang-up; incoming calls answered and hung up) |
| 3 | UX additions on existing data: favourites and display preferences kept locally, filters, the address book search on the server (settled text only), keyboard navigation | done (tested-offline; the address book search `tested-in-lab`) |
| 4 | QA against the specification and the reference images | done (see QA below) |

`commscraft --demo` shows the interface with made-up people at `beispiel.invalid` and never opens a
network connection; every demo conversation, call and meeting carries a visible demo mark, and the
status bar says "DEMO · not connected to a real server". Without `--demo` the sign-in screen
appears and the client signs in through the session engine.

## The session adapter (phase 2)

`apps/commscraft/src/session.rs` runs one worker thread per sign-in. It owns the TLS connection,
the session engine and the media threads of calls; the interface talks to it through two bounded
queues (commands 256, events 4096) and never waits for the network in a frame.

- **Sign-in:** Autodiscover and DNS SRV like the command-line tool (shared crate
  `commscraft-client`), NTLM with the password typed in the form, TLS always verified against the
  system trust store plus an optional `--ca-file`. A server outside the SIP domain is used only
  when its domain is listed in `~/.config/commscraft/trusted-domains`; the desktop client never
  asks in a dialog yet and says how to trust it. Kerberos can be tried first (a checkbox: the
  system's ticket, else the password; NTLM stays the fallback); TLS-DSK is offered by the
  command-line tool only.
- **Mapping:** engine events become typed `AppEvent`s (contacts, presence with the published
  display name, conversations, delivery, typing, roster, invitations, calls, meetings, notices);
  interface intents become engine commands. The interface's own conversation key is tied to the
  session's ID with `Engine::next_conversation_id`, so a failed opening reaches the right
  conversation. Groups (ours, joined meetings, an MCU that dials us) are shown as groups.
- **Calls:** the media thread of `commscraft-client::calls` (ICE, SRTP, G.722/G.711, ALSA); a call
  shows *connected* only after `CallAnswered`. Calls go only to SIP addresses picked in the
  interface – never to phone numbers – so the engine's emergency call path is not reached from
  here. The call tile *Transfer* sends a blind transfer (REFER) to an address picked in the address
  dialog; the result is reported, and the call ends only when the transfer succeeded. A
  transfer request of the other side is declined and a screen share offer refused, with a notice
  (the command-line tool offers both).
- **Address book search:** MS-DLX through the shared `commscraft-client::directory` with the
  registrar's NTLM; asked once the search text has rested for 600 ms; results are shown with
  their source and are not kept as contacts.
- **Not in the desktop client yet** (gated with a reason): video, viewing shared screens,
  meeting content, persistent chat.

`--sign-in sip:ADDRESS --user USER [--domain D] [--server H:P]` signs in at start with the
password from standard input; `--commands FILE` follows a file of test commands (`view`, `chat`,
`select`, `search`, `send`, `group`, `accept`, `call`, `transfer`, `answer`, `hangup`, `hold`, `resume`, `mute`,
`unmute`, `status`, `shot`, `quit`) that trigger exactly the actions of the interface – for the
owner's lab runs and screenshots. `COMMSCRAFT_VERBOSE=1` prints one line per SIP message (method
or status and CSeq, never headers).

## Honest capability gates

Every function the interface shows has a gate: **available** (supported and enabled), **preview**
(design with demo data only, no effect) or **unavailable** (disabled, the reason in the tooltip and
in Settings → "Functions and their state").

| Function | Gate today | Why |
|---|---|---|
| 1:1 chat, group chat, presence, address book search, audio call, hold, transfer (blind), meetings | available when signed in; preview in the demo (transfer: not shown in the demo) | through the session adapter |
| video call, persistent chat rooms, viewing shared screens, meeting content | unavailable | the command-line tool offers them; not in this interface yet |
| file transfer, formatted text, history search, read receipts | unavailable | not implemented (no public specification for file transfer; `text/plain` only; no archive or index; delivery is not reading) |

Delivery is shown as what it is: *sending*, *delivered* (the server or peer accepted the MESSAGE),
*not delivered (status)*, *not delivered to …* (IMDN in groups) or *delivery unknown*. There is no
"read" state. A call shows *connected* only after the session reported the answer; dialling and
ringing are separate states.

## Layout and theme

Top bar 66 px, rail 92 px, list panel 300 px, flexible main area, context panel 290 px (can be hidden,
closes by itself below 1100 px window width), status bar 28 px. Colours are tokens in
`crates/ui/src/theme.rs` (blue and slate: primary `#3267CC`, hover `#2456B6`, selection `#E8F0FF`,
outgoing `#E5EEFF`, ink `#24344F`, …). Status colours are semantic only and always come with a word.

Deviations from the reference images, on purpose:

- Secondary text is `#586A80` instead of `#64748B`: the latter reaches only 4.2:1 on the selection
  and message colours, the rule asks for 4.5:1 for normal text (tested in `theme.rs`). Success as
  text is `#177A59`; `#1E8E69` stays for dots.
- The rail shows two-letter monograms instead of drawn icons (no third-party icon sets).
- egui does not embolden fonts: bold text uses the system's bold face found through fontconfig.
- The brand icon in the top bar is the unchanged `assets/branding/CommSCraft-icon-128.png`.

## Fonts, accessibility, keyboard

- No font files are shipped: `fc-match` names the system's sans, bold and monospace faces (Inter,
  Noto Sans, DejaVu Sans, …); without any usable font the program says so and stops.
- Every interactive element has an accessible label (rows, rail buttons, call tiles, fields); the
  tests drive the interface through that accessibility tree. The Linux screen reader bridge
  (AccessKit over AT-SPI) is not built in yet: its adapter still uses zbus 5, whose `zvariant` and
  `zbus_names` crates RustSec lists as unmaintained (RUSTSEC-2026-0370 … 0372), and CI denies such
  warnings.
- Shortcuts: Ctrl+K search, Ctrl+N new conversation, Ctrl+1 … 4 views, Enter sends, Shift+Enter new
  line, Esc closes a dialog or the search, Tab and arrow keys move the focus. Drafts are kept per
  conversation; each conversation keeps its scroll position and only follows new messages while it
  is at the bottom.

## Local preferences (phase 3)

`~/.config/commscraft/desktop.conf` (mode 0600, written atomically) keeps favourites (SIP
addresses and group titles), high contrast, scale and whether the side panel is shown. No chat
content, presence or password is ever written there; the file is read bounded (64 KiB, 500
favourites) and malformed lines are skipped. The demo and screenshot runs never read or write it.

## QA (phase 4)

Checked against the owner's UI specification and acceptance checklist and against the three
reference designs (1440 × 900), with screenshots of the real build (above).

| Area | Result |
|---|---|
| Project integrity | `AGENTS.md`/`CLAUDE.md` unchanged; egui as decided; the protocol core has no interface dependency (`crates/ui` depends only on egui; the session engine gained one accessor, `next_conversation_id`); Linux desktop only; no alarm, situation, escalation or acknowledgement function (a test checks the screens for such words); the emergency call code is unchanged and not reachable from the desktop client (it calls SIP addresses only) |
| Layout | top bar 66, rail 92, list 300, context 290 (can be hidden, closes below 1100 px), status bar 28; screens for chats, contacts, calls, meetings, settings; usable at 1440 × 900, 1280 × 800 and at 200 % scale (test) |
| Colours | the blue and slate tokens in `theme.rs`; deviation: secondary text `#586A80` and success text `#177A59` for 4.5:1 (tested); status always with a word |
| Typography | system sans, bold and monospace faces through fontconfig (none bundled); sizes 22/19/14.5/13.5/11.5 pt; deviation from the references: the system face instead of Inter when Inter is not installed |
| Raster and icons | 8 px spacing, radius 8–10; deviation: two-letter monograms instead of drawn icons in the rail and call tiles |
| Interaction, keyboard, focus | Ctrl+K, Ctrl+N, Ctrl+1…4, Esc, Enter/Shift+Enter, Tab and arrow keys in the lists; visible focus rings on rows, rail buttons and tiles; drafts kept per conversation; scroll position kept per conversation |
| Data states | demo marked everywhere; delivery is never shown as read; a call is connected only after the answer (test); empty lists, offline, failed sign-in, a lost connection (with "sign in again"), a missing microphone (notice, the call goes on) all have words (tests and lab) |
| Security | no secrets in logs (the trace prints method or status and CSeq only), the password leaves the form once and is cleared, no TLS bypass, no end-to-end claim (the privacy text says TLS and SRTP are not end-to-end encryption), no chat content on disk |
| Accessibility | labels on all interactive elements (the tests use them); contrast tested; the Linux screen reader bridge is still missing (AccessKit's adapter on zbus 5 is listed as unmaintained) |

Open after phase 4 (in [`gaps.md`](../gaps.md) row 9): TLS-DSK sign-in, trusting a foreign server
domain in a dialog, the screen reader bridge, and video, viewing shared screens, meeting content
and persistent chat in the interface. Added after the QA: blind transfer (`tested-in-lab`) and
Kerberos as an option of the sign-in (tested offline; the lab signs in with NTLM).

## Data and security

- The interface keeps conversations in memory only, bounded (500 messages per conversation, 200
  conversations, 8 KiB per message shown, 10 000 contacts); nothing is written to disk.
- Text from the network is cleaned again before it is shown: control characters, bidirectional
  overrides and invisible format characters are removed, lengths are cut. Nothing is rendered as
  HTML, Markdown or rich text.
- The password leaves the sign-in form once, with the sign-in intent; the field and its undo memory
  are cleared, and the debug form of the intent hides it.
- Screenshots and tests use synthetic data only.

## Screenshots of the build

Made with `commscraft --demo --lang de --view VIEW --screenshot FILE` at 1440 × 900 (the program
saves its own first settled frame):

| Chats | Contacts | Calls |
|---|---|---|
| ![Chats](screenshots/chats.png) | ![Contacts](screenshots/contacts.png) | ![Calls](screenshots/calls.png) |

| Meetings | Settings | Sign-in | 1280 × 800, English |
|---|---|---|---|
| ![Meetings](screenshots/meetings.png) | ![Settings](screenshots/settings.png) | ![Sign-in](screenshots/signin.png) | ![1280](screenshots/chats-1280-en.png) |

| Search (sources named, message texts not searched) |
|---|
| ![Search](screenshots/search.png) |

## Checks

```
cargo fmt --all -- --check
cargo clippy -p commscraft-ui -p commscraft --all-targets -- -D warnings
cargo test -p commscraft-ui -p commscraft
cargo run -p commscraft -- --demo
```
