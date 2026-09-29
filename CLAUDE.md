# Zohara OS: instructions for Claude

Read these first, in order: `docs/HANDOFF-2026-09-29.md` (current state, how to build and test,
open items), then `GOTCHA.md` (traps that already cost time).

## Rules

- **Ask before every commit:** who gets credit (me / me + AI / AI) and which branch. "Only me"
  means no `Co-Authored-By` line. Never push to `main`/`master` unless told to.
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
