# Zohara OS: instructions for Claude

Read these first, in order: **`docs/HANDOFF-2026-10-10.md`** (2026-10-10: the cube, live wallpaper, dynamic desktops, sound balance, lock screen fix; channel state, verified vs not,
open items; notes in `zohara-settings/docs/FEATURES-2026-10-10.md` and `RESEARCH-2026-10-10.md`), **`docs/HANDOFF-2026-10-09.md`** (2026-10-09: the first long session on Zohara itself: channel state, what is verified and what is not,
open items; feature notes with file maps are in `zohara-settings/docs/FEATURES-2026-10-09.md`), **`docs/MOVING-TO-ZOHARA.md`** (2026-10-08: the owner replaced Zorin with Zohara; what is on the rescue stick, how to restore
the dev setup), **`docs/HOW-THE-OWNER-WORKS.md`** (the working agreement: read it before doing anything), `docs/READINESS-2026-10-08.md`,
`docs/HANDOFF-2026-10-06.md` (newest sections at the end), `docs/HANDOFF-2026-10-04.md`, `docs/HANDOFF-2026-09-29.md`, then `GOTCHA.md`
(traps that already cost time). The project skill `zohara-os-dev` has the short version.

## Rules

- **Ask before every commit:** who gets credit (me / me + AI / AI) and which branch. "Only me"
  means no `Co-Authored-By` line. Never push to `main`/`master` unless told to.
  **Development phase (owner's decision, 2026-10-05, still in force until they say otherwise):** commit and push
  straight to `main`/`master` as the owner (`Zohaib Baig`, only them, no `Co-Authored-By`) without asking each time;
  never build the ISO (`[skip ci]` on `zohara` commits). **Channels (2026-10-06):** a push to a source repo publishes to
  **alpha only**, by itself (the dev channel); beta and stable are moved on purpose from the release admin site. Still ask
  before anything that publishes to beta or stable, uses a bucket or a secret, or changes which key machines trust. See `docs/HANDOFF-2026-10-06.md`.
- **No Python in the OS.** Anything that ships is Rust or shell (`jq` for JSON). Python is fine
  for dev-machine helper scripts only, like `scripts/vm/vm.py`.
- **Everything user-facing must be clickable, not typeable.** The user finds pasting long
  commands hard. A flow that sends someone to a terminal is a bug: make it a button.
- **Do not claim something was built, copied, installed or tested without proof** (a hash of the
  freshly built file, a timestamp, a screenshot, a CI run). Compare against the new file's own
  hash, never two copies of the same file.
- Docs go in the repo they describe (Settings docs in `zohara-settings`, and so on).
- Never commit secrets: this repo is public. Keys stay out of git.

## Where things are

- ISO profile: `zohara-profile/`. CI builds the ISO on every push to master (25-35 min).
- Test in a VM before touching real hardware: `scripts/vm/` (QEMU + OVMF, drivable by script).
- Clone all sibling repos: `scripts/clone-all.sh`.
