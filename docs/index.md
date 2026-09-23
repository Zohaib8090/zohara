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

4. **[GSD Style Guide (GSD-STYLE.md)](./GSD-STYLE.md)**
   - Code style, formatting, and structural guidelines for the project.

5. **[Store Packages Guide (zohara_store_packages_guide.md)](./zohara_store_packages_guide.md)**
   - Detailed guide on how to package apps for the Zohara Store.
   - JSON schema and catalog management.

6. **[Build Commands (build command.txt)](./build%20command.txt)**
   - A quick scratchpad/reference for the manual Docker build commands.

---
*Note: If you are new to the project, start by reading `context.md` in its entirety to understand the nuances of the Archiso build process and the project's repository structure.*
