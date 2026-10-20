#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Zohara installer step: "Swap (no Hibernate)" must not offer Hibernate.

Calamares sizes the swap differently for "no Hibernate" and "with Hibernate", but its grubcfg module puts `resume=UUID=<swap>` on the
kernel line for ANY swap partition, so both choices came out with hibernation set up and a Hibernate button on the sign-in screen
(checked 2026-10-20 in a VM: same `resume=`, same `resume` hook, `CanHibernate = yes`, only the swap size differed).

Runs after grubcfg (which writes /etc/default/grub) and before bootloader (which runs grub-mkconfig). For the choices "No swap" and
"Swap (no Hibernate)" it
  * takes `resume=...` out of GRUB_CMDLINE_LINUX_DEFAULT, and
  * writes /etc/systemd/sleep.conf.d/10-zohara-no-hibernate.conf (`AllowHibernation=no`) so the sign-in screen and the desktop's power
    menu do not show Hibernate.
"Swap (with Hibernate)" and the other choices are left as they are. This step never stops an install: any problem is logged and skipped.
"""
import os
import re

import libcalamares

NO_HIBERNATE_CHOICES = ("none", "small")

DROPIN = """# Written by the Zohara installer because "Swap (no Hibernate)" (or no swap) was chosen.
# To get Hibernate: delete this file, put resume=UUID=<swap> on the kernel line, and make sure that swap is as big as the RAM.
[Sleep]
AllowHibernation=no
AllowSuspendThenHibernate=no
AllowHybridSleep=no
"""


def strip_resume(text):
    """Remove `resume=...` (and `resume_offset=...`) from the GRUB_CMDLINE_LINUX* lines of /etc/default/grub; nothing else changes."""
    out = []
    for line in text.splitlines(keepends=True):
        if line.lstrip().startswith("GRUB_CMDLINE_LINUX"):
            # the option and the one space after it; then no space left in front of the closing quote
            line = re.sub(r"(?<=['\"\s])resume(?:_offset)?=[^\s'\"]*[ \t]?", "", line)
            line = re.sub(r"[ \t]+(['\"])[ \t]*(\r?\n?)$", r"\1\2", line)
        out.append(line)
    return "".join(out)


def wanted(choices):
    """True when this install should not offer Hibernate."""
    return bool(choices) and choices.get("swap") in NO_HIBERNATE_CHOICES


def apply(root, choices):
    """Change the target system at `root`; returns a short description of what was done."""
    if not wanted(choices):
        return "swap choice %r: hibernation left as it is" % (choices or {}).get("swap")
    done = []
    grub = os.path.join(root, "etc/default/grub")
    if os.path.exists(grub):
        with open(grub, encoding="utf-8") as f:
            text = f.read()
        new = strip_resume(text)
        if new != text:
            with open(grub, "w", encoding="utf-8") as f:
                f.write(new)
            done.append("removed resume= from /etc/default/grub")
    d = os.path.join(root, "etc/systemd/sleep.conf.d")
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, "10-zohara-no-hibernate.conf"), "w", encoding="utf-8") as f:
        f.write(DROPIN)
    done.append("wrote sleep.conf.d/10-zohara-no-hibernate.conf")
    return "; ".join(done)


def pretty_name():
    return "Hibernation settings"


def run():
    try:
        gs = libcalamares.globalstorage
        root = gs.value("rootMountPoint")
        choices = gs.value("partitionChoices")
        libcalamares.utils.debug("hibernatefix: " + apply(root, choices))
    except Exception as e:  # never stop an install for this
        libcalamares.utils.warning("hibernatefix skipped: %r" % (e,))
    return None
