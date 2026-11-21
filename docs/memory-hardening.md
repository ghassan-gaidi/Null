# Memory & endpoint hardening

Everything that keeps keys out of disk, out of dumps, and out of easy
grasp when a machine (or its owner) is compromised. Implementation:
`crates/null-memory` + hardening reached from `null-cli` and `null-tui`.

## 1. RAM-only, zero disk writes

- No database, no config files, no session logs, no swap-backed key
  storage. `null` writes nothing to disk apart from what the build does.
- Every key container implements `zeroize::ZeroizeOnDrop`; small secrets
  (root/chain/message keys) are derived on the stack where possible and
  wiped when the frame is done (`perf(crypto)`: ratchet keys on the
  stack, no-op Drop removed).
- The skipped-sign chain-key cache (`MAX_SKIP = 200`) and retained rekeys
  (`RETAINED_REKEYS = 8`) live in RAM and are wiped on drop.

## 2. Kernel-backed protections (Linux-first)

Applied once at startup by `null_memory::harden_process()`:

- `PR_SET_DUMPABLE = 0`: no core dump, and a debugger cannot attach to a
  live `null` without privilege (same-uid ptrace is refused).
- `RLIMIT_CORE = 0` (soft and hard): no core file even if the dumpable
  flag were flipped later. Children inherit the limit.

A warning is printed if either step fails.

Available but **not wired to any secret yet**:

- `HardenedBuffer`: page-aligned, `mlock`ed, `MADV_DONTDUMP`, zeroized
  and unlocked on drop. Ratchet, identity and Kyber keys do not live in
  one, so they are ordinary memory and **can be swapped** (see Known gaps).

Non-Linux builds get no process hardening (`HardeningReport` is all
false). There is no macOS/Windows `VirtualLock`/`mach_vm_wire` code.

## 3. The exit path

`null_memory::clear_terminal` runs on any of:

- SIGINT (Ctrl-C), SIGTERM, SIGHUP (signal_hook handlers),
- the Rust panic hook (clears the screen, then prints the panic and
  unwinds),
- `/quit`, stdin EOF, peer goodbye, peer FIN/RST,
- the dead-man switch,
- the duress PIN,
- the USBGuard trigger (new `/dev` node).

Sequence: clear the screen and scrollback with one `write(2)`, then
exit. `process::exit` and `_exit` skip destructors, so **this path does
not zeroize keys**; the kernel reclaims the pages. Exit codes: `0` for
deliberate exits (`/quit`, Ctrl-C, EOF, peer goodbye, duress PIN), `143`
for SIGTERM, `129` for SIGHUP, `2` for the USBGuard trigger; error paths
propagate a nonzero status. The SIGTERM/SIGHUP handlers call only
`write(2)` and `_exit(2)`, which are async-signal-safe.

## 4. Terminal hygiene

- Enters the alternate screen on startup, disables scrollback/selection
  history where the terminal permits, and kills scrollback on exit.
- Although it reads input from `/dev/tty` (not stdin history), real
  terminal keyloggers are a stated mitigation-only problem — see
  `--secure-input` below.
- Bracketed paste stays off by default; the TUI prompts explicitly when a
  paste is offered.

## 5. Clipboard sanitization

- A copy (`/copy` or TUI copy action) schedules a background clear after
  `CLIPBOARD_CLEAR_SECS` seconds (**5**), on `wl-copy`/`xclip`/
  `pbcopy`/`termux` families per OS.
- Wayland `wl-copy` zombies are reaped (fixed); the clear thread exists so
  the UI never blocks on it.
- Clipboard-manager presence is detected and the user is warned (a manager
  like CopyQ/Ditto defeats any app-level clear).

## 6. Secure input (`--secure-input`)

- Root-only: grabs `/dev/input/event*` via a minimal evdev keymap and
  reads keystrokes directly, bypassing terminal keyloggers
  (`null-cli/src/secure_input.rs`).
- Without root it warns and falls back to `/dev/tty`; in TUI mode it is
  ignored (the TUI owns the keyboard) with a warning.
- Fallback cadence: 1 s retry on read errors.

## 7. Duress, decoy, lock, dead-man, USBGuard

| Feature | Default | Behavior |
|---|---|---|
| Auto-lock (`--auto-lock-secs`) | 1800 s (30 min) idle; only active when lock PINs are set | Locks the UI; PIN to resume |
| Dead-man switch (`--dead-man-secs`) | **Disabled (0)** — the code-level baseline constant is `DEAD_MAN_SWITCH_SECS = 1800`, but shipping it armed by default would wipe laptops on idle; enabling is an explicit operator decision | Clear terminal + exit after N idle seconds |
| Lock / duress PIN | None. Set `NULL_LOCK_PIN` and `NULL_DURESS_PIN` (at least 4 characters, different); without them the lock screen and auto-lock are unavailable. Loopback demo only: `1234` / `0000` | The duress PIN takes the exit path in section 3 (terminal clear + exit 0); it does not zeroize keys |
| Decoy mode (`--safe`) | off | Benign IRC-like interface; the real session hides behind `/unlock` |
| USBGuard (`--usbguard`) | off | Polls `/dev`; any new node → clear terminal + exit 2 |

## 8. HSM tiers

- **Tier 1 — software, shipped**: `SoftwareHsm::bind_local(secret,
  context) = SHA384("Null-v2.0-hsm-bind:" ‖ context ‖ machine_id ‖
  secret)`, machine id from `/etc/machine-id`. Device-bound local *identity
  material* only.
- **Critical rule**: the *shared ratchet root* is **never** mixed with
  device-local material — peers could not converge on a session key.
  Device binding guards local secrets (identities, device ids), and the
  handshake/ratchet root is derived purely from protocol material.
- **Tier 2 — probe (`--hsm check`)**: detects TPM2 (`/dev/tpmrm0`,
  `/dev/tpm0` → `tpm2`), YubiKey (`ykman`/`ykchalresp` →
  `yubikey-slot2-hmac`), and macOS Secure Enclave. Detection reports
  honestly instead of silently downgrading; **native key operations need
  the vendor stacks (tpm2-tss, ykman, SE SDK), which are not yet linked**
  (`docs/security-posture.md`).

## 9. Sleep / hibernation

`sleep_protection_status()` reports whether the OS will write RAM to disk
before suspend (systemd-logind inhibitor logic on Linux; macOS/Windows
hooks scaffolded). The CLI prints this at startup so an operator running
`null` on a laptop sees the hibernation risk before it matters. Inline
pre-suspend wiping is roadmap work.

## 10. Threat-model honesty

These are **mitigations, not proofs**:

- A live-process memory scraper (user-level malware already running)
  wins; anti-dump/`mlock` only raise the bar (`docs/threat-model.md`).
- Root/kernel malware is out of scope for any messenger.
- Cold boot with power retained is made expensive (RAM-only, wipe on
  signal), not impossible.
- TPM/YubiKey/Secure Enclave *operations* are not shipped; the probe
  cannot be relied on as a key-isolation boundary today.

## Known gaps

- Key material is not held in `HardenedBuffer` or any locked memory, so
  swap can capture it. Mitigation today: no swap, or encrypted swap.
- No zeroization on exit: every deliberate exit and both signal handlers
  end in `process::exit`/`_exit`, which skip `Drop`.
- `mlock` failure (low `RLIMIT_MEMLOCK`) is a warning, not an error;
  check `HardenedBuffer::is_locked()` where the guarantee matters.
- `mlockall` is not used: with a typical 8 MiB memlock limit it would make
  ordinary allocations fail.
- Cold boot and live-process memory scraping are out of scope
  (`docs/threat-model.md`).
