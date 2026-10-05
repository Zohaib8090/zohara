# Secure Boot for Zohara OS: plan

Written 2026-10-05. Nothing in this plan is built yet. Facts below were read from this repository or checked
against Arch's package repositories on that day; things that are **unknown** are marked as such.

## Why

Today the README, the website and the SourceForge page tell people to **turn Secure Boot off**. Many new laptops ship
with it on, and some work or school machines cannot switch it off. Every Zohara bootloader and kernel is unsigned, so
those machines refuse to start Zohara at all. Supporting Secure Boot widens who can install Zohara and keeps the
protection Secure Boot gives (nobody can swap the bootloader or kernel for a malicious one unnoticed).

## Owner's decision (2026-10-05): users must not do any key steps

The target is the Ubuntu experience: plug in the USB, boot, install, with no "Enroll key" screen. That is **option B
below** and it cannot be done by the project alone: the signature that makes Ubuntu seamless is Microsoft's, on a
shim that has the distribution's key built into it. What that needs, in order (details and fees must be checked
against the current rules before starting; they change):

1. **An identity Microsoft accepts.** Signing requests go through Microsoft's Partner Center, which verifies the
   account holder (usually a registered organisation or a verified individual) and needs a code-signing certificate
   (EV certificates are bought from a certificate authority, around a few hundred dollars a year).
2. **A shim-review submission.** The community review at github.com/rhboot/shim-review checks that the shim is built
   reproducibly from source with Zohara's certificate inside, that GRUB and the kernel meet the security rules
   (current shim, SBAT data, GRUB patched for known holes, kernel lockdown on, no way to load unsigned code), and that
   the project can ship fixes quickly. Typical time: weeks to months, with back-and-forth.
3. **Microsoft signs that shim.** After approval; it can take more weeks.
4. **Everything after shim is Zohara's job** and must be done to the review's standard: signed GRUB, signed kernel,
   a way to keep both up to date, and no per-user keys. See "What the chain needs" below.

Until then the honest instructions stay "turn Secure Boot off". An optional in-between (option A, users enrol the key
once) can be offered for people who cannot turn Secure Boot off, but it is not the goal.

**NVIDIA without any prompt** needs a different design than DKMS: Zohara would build its own kernel and the matching
NVIDIA modules in CI and sign the modules with a key whose certificate is built into that kernel, so the kernel accepts
them with no enrolment. (Ubuntu itself asks for a one-time password when a user installs a DKMS driver under Secure
Boot; it avoids prompts only for drivers it has pre-built and signed.) That means Zohara maintaining its own kernel
build. The cheaper stopgap is the open-source driver when Secure Boot is on.

## What Secure Boot checks, in one paragraph

The firmware holds a list of trusted keys, normally Microsoft's. It starts only an EFI program signed by one of them.
That program then decides what it starts next. Linux distributions use a small loader called **shim**, signed by
Microsoft; shim trusts the distribution's own key (built in) and any key the user has enrolled in the **MOK** list
(Machine Owner Key), and starts GRUB, which starts the kernel. Each step must be signed by a key shim trusts.
Without a Microsoft-signed shim, the only other way is for the owner to put their own keys into the firmware.

## How Zohara boots today

| Stage | Live USB (ISO) | Installed system |
|---|---|---|
| Firmware starts | `systemd-boot` (archiso `uefi.systemd-boot`; `profiledef.sh` bootmodes `bios.syslinux`, `uefi.systemd-boot`) | GRUB, installed by Calamares (`bootloader` module), with `grub-btrfs` snapshot entries |
| Then loads | `vmlinuz-linux-zen` + `initramfs-linux-zen.img` straight from the ISO | the same kernel from `/boot` on Btrfs, plus initramfs |
| Out-of-tree drivers | none built at boot | `nvidia-open-dkms` is **built on the user's machine** by DKMS for `linux-zen` (there is no prebuilt `nvidia-open` for zen in Arch: only for `linux` and `linux-lts`) |

Every file in that chain is unsigned.

## The options

| | What | Cost | User sees |
|---|---|---|---|
| **A. shim + MOK (recommended first step)** | Microsoft-signed `shim`, then GRUB and the kernel signed with a **Zohara Secure Boot key**; the user enrols that key once | free; engineering work | one blue "Enroll key" screen on the first boot, with a one-time password |
| B. Get Zohara's own shim signed by Microsoft | Zohara's key is built into shim, so there is no enrolment screen | a company-like review, paperwork, a signing fee, a reproducible-build requirement, months | nothing, it just boots |
| C. Owner enrols keys in the firmware (sbctl) | the user puts Zohara's key in the firmware's `db` | none, but expert-only | a trip into the BIOS key menus |
| D. Keep "turn Secure Boot off" | status quo | none | a BIOS visit |

Recommendation (revised after the owner's decision above): the **goal is B**; **D stays the instruction** until B is
done; **A** is an optional step for people who cannot turn Secure Boot off. All the engineering in this plan is needed
for B as well, so building it is not wasted while the Microsoft process runs.

## What the chain needs (for A and for B), piece by piece

### 1. The Zohara Secure Boot key
* An RSA-3072 (or 2048) X.509 certificate + private key, **separate from the package-signing key**. Shim reads RSA
  certificates, not the ed25519 key used for pacman.
* The private key stays offline (a USB stick or a hardware token). It signs GRUB and the kernel in CI or by hand;
  the key never goes onto users' machines.
* Lifetime: long (10 years); losing it or rotating it means every machine must enrol a new certificate.

### 2. shim
* **Unknown / to research:** `extra/shim 16.1` exists in Arch, but Arch builds it **unsigned**. A shim that stock
  firmware accepts must carry Microsoft's signature, which means taking the already-signed shim from a distribution that
  has one (the AUR `shim-signed` package does this). Check its license and where the binary comes from before
  redistributing it in the ISO.
* Shim loads `grubx64.efi` from its own folder and `mmx64.efi` (MokManager) for enrolment.

### 3. GRUB
* Must be installed with the modules **embedded** (`grub-install --modules=...`): under Secure Boot GRUB will not load
  extra modules from disk. The list has to cover Btrfs, `part_gpt`, `gzio`, `search`, `normal`, `linux`,
  `gfxterm` and whatever `grub-btrfs` and the theme use (today's embedded list is **not known**; take it from a
  working install).
* GRUB is signed with the Zohara key. It must NOT be installed with `--disable-shim-lock`, because shim's verifier
  is what checks the kernel.
* Arch's `grub` package supports `--sbat`; shim 16 checks SBAT revocation data, so GRUB needs a valid SBAT section.
* Calamares' `bootloader` module runs `grub-install`; it needs a post step (or `shellprocess`) that installs
  `shimx64.efi` as `BOOTX64.EFI`/the `Zohara_OS` entry, copies `mmx64.efi`, and signs GRUB. The existing repair script
  `scripts/repair-boot-entry.sh` and `GOTCHA.md` (Dell: premature end of file) show how fragile this step already is:
  it must be tested on the Dell too.

### 4. The kernel
* `vmlinuz-linux-zen` is signed with the Zohara key (`sbsign`).
* A **pacman hook** re-signs it on every kernel update, **before** the bootloader entries are regenerated. If the hook
  fails the update must stop, because an unsigned kernel under Secure Boot means a machine that does not start.
  (`sbctl` automates the hook but expects keys on the machine; here the key is offline, so Zohara needs its own hook
  that signs with a key handed to it or, better, ships the kernel already signed, see 8.)
* Snapshots (`grub-btrfs`) boot kernels copied into snapshots; those were signed when copied, so they work, but old
  unsigned kernels from before the switch will not.
* **The initramfs is not covered by Secure Boot.** An attacker with disk access could change it. Closing that gap needs
  a **unified kernel image** (UKI: kernel + initramfs + command line in one signed file, `systemd-ukify` is in Arch).
  That is a bigger change to boot, snapshots and the installer; treat it as a second phase.

### 5. Kernel lockdown and the NVIDIA driver (the awkward part)
* With Secure Boot on, the kernel normally switches on **lockdown**, which refuses unsigned kernel modules. DKMS builds
  the NVIDIA module on the user's machine, so it is unsigned and would not load: no NVIDIA graphics.
* DKMS can sign modules with a key it is given (`/etc/dkms/framework.conf`: `mok_signing_key`, `mok_certificate`). The
  module key must be **per machine**, created during install, and its certificate **enrolled in MOK** alongside Zohara's.
  The user then enrols two certificates in the same blue screen.
* Alternatives to research: a prebuilt, Zohara-signed NVIDIA package for `linux-zen` (needs a CI build per kernel
  release), or defaulting NVIDIA machines to the open-source driver (the ISO already has an "Open Source Drivers" entry)
  when Secure Boot is on.
* **Unknown:** whether `linux-zen` in Arch has `CONFIG_LOCK_DOWN_IN_EFI_SECURE_BOOT` on. Check `/boot/config` or
  `zcat /proc/config.gz` on a built system.

### 6. The live ISO
* Today the ISO's UEFI loader is `systemd-boot`, unsigned. Under Secure Boot it needs the same chain: shim, then a signed
  loader (GRUB or a signed systemd-boot), then the signed kernel. archiso has no ready-made shim mode, so the ESP image
  has to be post-processed by a script in the ISO build.
* The user must enrol the Zohara key **before the live system starts** (MokManager appears on that first boot of the
  USB). MOK is stored in the machine's firmware memory, so the installed system then boots without asking again.
* The Welcome app and the website need a short, clear "what the blue screen is and what to press" section with photos.

### 7. Tests (these can be done on this laptop)
* `/usr/share/OVMF/OVMF_CODE_4M.secboot.fd` and `OVMF_VARS_4M.ms.fd` are installed here: QEMU can run a VM with Secure
  Boot **on** and Microsoft's keys, which is the same situation as a real laptop. `scripts/vm/` needs a variant that
  uses them.
* Test list: (a) the current ISO is refused (proves the test works), (b) the new ISO shows MokManager, (c) after
  enrolling it boots to the desktop, (d) install to disk and reboot, (e) a kernel update re-signs and still boots,
  (f) a Btrfs snapshot entry boots, (g) tampered GRUB is refused, (h) NVIDIA path on real hardware.
* Real hardware: at least the Dell Latitude 5330 (known to be picky about boot entries) and one other laptop.

### 8. Where the signing happens
* Simplest: CI signs GRUB and the kernel with the offline key during the ISO build and in a "signed boot files" package
  (`zohara-boot-signed`, containing the signed `grubx64.efi` and the signed kernel), so users' machines never hold the
  private key. A kernel update then arrives as a Zohara package rather than straight from Arch.
* This is a real cost: Zohara would **rebuild/re-sign `linux-zen` on every Arch kernel release**. Alternative: sign on
  each machine with a per-machine key that the user enrols (the `sbctl` model) and accept that the key is on the
  machine. Decide before building (see questions).

## Risks

* **Unbootable machines.** A wrong signature, a missing GRUB module or a stale SBAT entry leaves a machine that does
  not start. Mitigations: test in QEMU first, keep the unsigned path working with Secure Boot off, change the real
  laptop only after the VM passes, keep a recovery USB.
* **Key loss or leak.** Lost: all machines must enrol a new certificate. Leaked: anyone can sign a bootable OS that
  your users' machines will trust. Keep it offline and back it up.
* **The MOK screen is scary** for the people Zohara targets. It needs clear instructions and ideally a one-page guide
  with photos.
* **Arch churn.** Kernel, GRUB and shim updates all touch this chain.

## Phases (each has a check before the next)

| # | Work | Check |
|---|---|---|
| SB-0 | Decisions below; research the two unknowns (signed shim source and license; lockdown setting) | written answers in this file |
| SB-1 | QEMU Secure Boot test setup in `scripts/vm/`; prove today's ISO is refused | a failing boot screenshot, a passing script |
| SB-2 | Make the Zohara Secure Boot key (offline); sign GRUB + kernel by hand in a VM; boot with shim + MOK enrolment | VM with Secure Boot on reaches the login screen |
| SB-3 | Installed system: Calamares post step, kernel-update hook (or signed-kernel package), embedded GRUB modules | install in the VM, reboot, kernel update, snapshot boot |
| SB-4 | Live ISO with shim; ISO build script changes | the ISO boots in the Secure Boot VM after enrolment |
| SB-5 | NVIDIA path (per-machine DKMS key or alternative) | NVIDIA machine boots with Secure Boot on |
| SB-6 | Real hardware (Dell + another); docs, website and Welcome text; drop "turn Secure Boot off" | two laptops boot with Secure Boot on |
| SB-7 | Optional: unified kernel image so the initramfs is covered too | a changed initramfs is refused |

## Questions for the owner

0. **Are you willing to start the Microsoft signing process** (identity check, a certificate, a review that takes weeks
   to months)? Without it Zohara cannot boot with Secure Boot on without a user step.

1. **Where does signing happen?** CI signs everything with an offline key and Zohara ships a signed kernel package
   (more work, safest for users), or each machine signs with its own key (less work, the key lives on the machine)?
2. **NVIDIA:** is it acceptable that NVIDIA machines use the open-source driver while Secure Boot is on until a proper
   solution exists?
3. **Key storage:** a USB stick kept offline, or do you want to buy a hardware token (YubiKey)?
4. **Hardware:** which real laptops can be used for testing besides the Dell?
5. **Order:** package signing is done up to "machines require signatures". Secure Boot is the next large item; is this
   the right moment, or should the Store/Settings bugs and the ISO promote test come first?
