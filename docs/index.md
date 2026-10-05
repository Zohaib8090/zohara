# Zohara OS Documentation

Welcome to the Zohara OS documentation. This directory contains the complete reference and guides for the Zohara OS project, including its architecture, build pipeline, and core components.

## Table of Contents

1. **[Context & Architecture (context.md)](./context.md)**
   - The master "Single Source of Truth" for Zohara OS.
   - Core components breakdown (Settings, Packages, Store).
   - ISO build pipeline architecture.
   - Crucial verified facts and historical fixes (must-read for developers).

2. **[Technical Reference (DOCUMENTATION.md)](./DOCUMENTATION.md)**
   - Overview and project goals.
   - Deep dive into `zohara-settings-rs` multi-threading & async GLib architecture.
   - Store & version management protocol.
   - Quick ISO generation commands and troubleshooting.

3. **[Project Rules (PROJECT_RULES.md)](./PROJECT_RULES.md)**
   - Guidelines and guardrails for modifying the Zohara OS codebase.
   - AI agent instructions and repository standards.

4. **[Handoff, 2026-09-29 (HANDOFF-2026-09-29.md)](./HANDOFF-2026-09-29.md)** and
   **[Handoff, 2026-09-30 (HANDOFF-2026-09-30.md)](./HANDOFF-2026-09-30.md)** and
   **[Handoff, 2026-10-04 (HANDOFF-2026-10-04.md)](./HANDOFF-2026-10-04.md)** (ISO hosting, website, the release admin site)
   - Current state, what was tested in the VM, what is fixed, and the open items. Read these first
     after `CLAUDE.md`; the newer one updates the older one.

   - Plans: **[Secure Boot (SECURE-BOOT-PLAN.md)](./SECURE-BOOT-PLAN.md)** (not built yet) and package signing
     (`docs/SIGNING.md` in the `zohara-packages` repo, key made and keyring published, signing switched off).

5. **[Store Packages Guide (zohara_store_packages_guide.md)](./zohara_store_packages_guide.md)**
   - Detailed guide on how to package apps for the Zohara Store.
   - JSON schema and catalog management.

6. **[Build Commands (build command.txt)](./build%20command.txt)**
   - A quick scratchpad/reference for the manual Docker build commands.

7. **[VM test checklist (VM-TEST-CHECKLIST.md)](./VM-TEST-CHECKLIST.md)** and the VM rig in
   `../scripts/vm/README.md`
   - How to install and boot the ISO in QEMU and check the installed system.

Traps that already cost time are in [`../GOTCHA.md`](../GOTCHA.md).

---
*Note: If you are new to the project, start by reading `context.md` in its entirety to understand the nuances of the Archiso build process and the project's repository structure.*
