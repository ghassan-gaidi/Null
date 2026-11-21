//! Volatile-RAM process and buffer hardening, Linux-first (§7).
//!
//! What this crate does today:
//! - [`harden_process`]: `PR_SET_DUMPABLE=0` and `RLIMIT_CORE=0` (no core
//!   dumps, no same-uid ptrace attach under default Yama). Call once at
//!   startup.
//! - [`HardenedBuffer`]: a page-aligned, `mlock`ed, `MADV_DONTDUMP` region
//!   that is zeroized and unlocked on drop.
//! - [`clear_terminal`]: async-signal-safe screen and scrollback clear.
//! - systemd-logind sleep inhibitor hint (best-effort).
//!
//! What it does not do: nothing in the workspace stores ratchet or identity
//! keys in a [`HardenedBuffer`] yet, so those secrets live in ordinary
//! heap/stack memory (zeroized on drop where the owning type implements it)
//! and can be swapped. `docs/memory-hardening.md` lists the open gaps.

use null_core::{NullError, Result};
use std::alloc::{alloc_zeroed, dealloc, Layout};
use zeroize::Zeroize;

#[cfg(target_os = "linux")]
mod os {
    pub use libc::{
        c_void, getrlimit, madvise, mlock, munlock, prctl, rlimit, setrlimit, sysconf, write,
        MADV_DONTDUMP, MADV_WILLNEED, PR_GET_DUMPABLE, PR_SET_DUMPABLE, RLIMIT_CORE,
        RLIMIT_MEMLOCK, RLIM_INFINITY, STDOUT_FILENO, _SC_PAGESIZE,
    };
}

/// Outcome of [`harden_process`]. A `false` field means the step failed or is
/// not supported on this OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardeningReport {
    /// `PR_SET_DUMPABLE` is 0: no core dump, no ptrace attach by same-uid
    /// processes that lack `CAP_SYS_PTRACE`.
    pub non_dumpable: bool,
    /// `RLIMIT_CORE` is 0 (soft and hard) for this process and its children.
    pub core_dumps_disabled: bool,
    /// `RLIMIT_MEMLOCK` soft limit in bytes (`u64::MAX` = unlimited, 0 =
    /// unknown). Bounds how much [`HardenedBuffer`] memory can be locked.
    pub memlock_limit: u64,
}

impl HardeningReport {
    /// Both process-wide protections took effect.
    pub fn is_complete(&self) -> bool {
        self.non_dumpable && self.core_dumps_disabled
    }
}

/// Process-wide dump protection. Call once, before any secret exists.
///
/// The core-size limit cannot be raised again by an unprivileged process, so
/// this is one-way for the life of the process.
pub fn harden_process() -> HardeningReport {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: plain syscalls; the pointers refer to live locals.
        let non_dumpable = unsafe { os::prctl(os::PR_SET_DUMPABLE as _, 0, 0, 0, 0) } == 0
            && unsafe { os::prctl(os::PR_GET_DUMPABLE as _, 0, 0, 0, 0) } == 0;
        let zero = os::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        let core_dumps_disabled = unsafe { os::setrlimit(os::RLIMIT_CORE, &zero) } == 0;
        let mut ml = os::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // rlim_t is not u64 on every Linux target, so the cast stays.
        #[allow(clippy::unnecessary_cast)]
        let memlock_limit = if unsafe { os::getrlimit(os::RLIMIT_MEMLOCK, &mut ml) } == 0 {
            if ml.rlim_cur == os::RLIM_INFINITY {
                u64::MAX
            } else {
                ml.rlim_cur as u64
            }
        } else {
            0
        };
        HardeningReport {
            non_dumpable,
            core_dumps_disabled,
            memlock_limit,
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        HardeningReport {
            non_dumpable: false,
            core_dumps_disabled: false,
            memlock_limit: 0,
        }
    }
}

fn page_size() -> usize {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: sysconf has no preconditions.
        let p = unsafe { os::sysconf(os::_SC_PAGESIZE) };
        if p > 0 {
            return p as usize;
        }
    }
    4096
}

/// Page-aligned, locked, non-dumpable heap region. Zeroized + unlocked on
/// drop.
///
/// The region is whole pages and page-aligned on purpose: `mlock`/`munlock`
/// work per page and are not reference counted, so a buffer sharing a page
/// with other allocations would unlock their pages when it is dropped.
pub struct HardenedBuffer {
    ptr: *mut u8,
    layout: Layout,
    len: usize,
    locked: bool,
}

// SAFETY: the buffer owns its allocation exclusively; `&self` only reads it.
unsafe impl Send for HardenedBuffer {}
unsafe impl Sync for HardenedBuffer {}

impl HardenedBuffer {
    pub fn new(len: usize) -> Result<Self> {
        if len == 0 {
            return Err(NullError::Memory("zero len".into()));
        }
        let page = page_size();
        let size = len
            .checked_next_multiple_of(page)
            .ok_or_else(|| NullError::Memory("size overflow".into()))?;
        let layout = Layout::from_size_align(size, page)
            .map_err(|e| NullError::Memory(format!("layout: {e}")))?;
        // SAFETY: layout has non-zero size.
        let ptr = unsafe { alloc_zeroed(layout) };
        if ptr.is_null() {
            return Err(NullError::Memory("alloc failed".into()));
        }
        let mut b = Self {
            ptr,
            layout,
            len,
            locked: false,
        };
        b.locked = b.lock();
        b.advise();
        Ok(b)
    }

    /// Whether `mlock` succeeded. When false the region is ordinary
    /// pageable memory (RLIMIT_MEMLOCK too low, or non-Linux).
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: ptr is a live allocation of at least `len` bytes.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: as above, and `&mut self` guarantees exclusivity.
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    fn lock(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            // SAFETY: the range is this buffer's own allocation.
            let r = unsafe { os::mlock(self.ptr as *const os::c_void, self.layout.size()) };
            if r != 0 {
                // RLIMIT_MEMLOCK can be low (containers, default user
                // limits). Callers that need the guarantee must check
                // `is_locked()`.
                eprintln!(
                    "[null-memory] WARN: mlock failed ({}), buffer is pageable",
                    std::io::Error::last_os_error()
                );
            }
            r == 0
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    fn advise(&self) {
        #[cfg(target_os = "linux")]
        // SAFETY: the range is this buffer's own allocation. Failures are
        // advisory only and ignored.
        unsafe {
            os::madvise(
                self.ptr as *mut os::c_void,
                self.layout.size(),
                os::MADV_WILLNEED,
            );
            os::madvise(
                self.ptr as *mut os::c_void,
                self.layout.size(),
                os::MADV_DONTDUMP,
            );
        }
    }

    /// Zeroize the whole region, padding included. One volatile zeroing
    /// pass is sufficient for RAM; multi-pass overwrite schemes target
    /// magnetic media. The region stays locked until drop.
    pub fn panic_wipe(&mut self) {
        // SAFETY: ptr is a live allocation of `layout.size()` bytes.
        let all = unsafe { std::slice::from_raw_parts_mut(self.ptr, self.layout.size()) };
        all.zeroize();
    }
}

impl Drop for HardenedBuffer {
    fn drop(&mut self) {
        self.panic_wipe();
        #[cfg(target_os = "linux")]
        if self.locked {
            // SAFETY: unlocks exactly the pages this buffer locked, and no
            // other allocation shares them.
            unsafe { os::munlock(self.ptr as *const os::c_void, self.layout.size()) };
        }
        // SAFETY: ptr/layout come from the matching alloc_zeroed.
        unsafe { dealloc(self.ptr, self.layout) };
    }
}

const CLEAR_SEQ: &[u8] = b"\x1b[3J\x1b[H\x1b[2J";

/// Clear the screen and scrollback (ESC[3J ESC[H ESC[2J).
///
/// A single `write(2)` on stdout, so it is safe to call from a signal
/// handler: no allocation, no locks. This clears the terminal only. It does
/// not touch key material.
pub fn clear_terminal() {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: valid pointer and length of a 'static buffer. A short or
        // failed write is acceptable for a best-effort screen clear.
        let _ = unsafe {
            os::write(
                os::STDOUT_FILENO,
                CLEAR_SEQ.as_ptr() as *const os::c_void,
                CLEAR_SEQ.len(),
            )
        };
    }
    #[cfg(not(target_os = "linux"))]
    {
        use std::io::Write;
        let mut out = std::io::stdout();
        let _ = out.write_all(CLEAR_SEQ);
        let _ = out.flush();
    }
}

/// Best-effort systemd sleep inhibitor: returns guard string describing
/// state. Real inhibitor fd via `systemd-inhibit` is wired in null-cli so
/// this crate stays dependency-free.
pub fn sleep_protection_status() -> &'static str {
    if std::path::Path::new("/run/systemd/seats").exists() {
        "systemd-logind present: caller should hold delay lock (see null-cli)"
    } else {
        "no systemd: warn user if hibernation enabled"
    }
}

// ---------------------------------------------------------------------------
// HSM integration (§7.5).
//
// The shared ratchet root is NEVER mixed with device-local HSM material:
// peers could not converge (each device differs). The HSM instead guards
// LOCAL secrets — long-term identity keys and sealed backups — while the
// Tier-1 default keeps everything in volatile RAM.
// ---------------------------------------------------------------------------

/// Hardware (or software) backend guarding local secrets.
pub trait HsmBackend: Send + Sync {
    /// Stable label recorded in logs/diagnostics (never key material).
    fn name(&self) -> &'static str;
    /// True when the backend can actually isolate keys on this machine.
    fn isolates_keys(&self) -> bool;
    /// Device-bind a local secret (e.g. identity seed): domain-separated
    /// mixing with device-stable material. Deterministic per device.
    fn bind_local(&self, secret: &[u8], context: &[u8]) -> [u8; 48];
}

/// Tier 1 (default): RAM-only keys, no isolation, device binding via
/// `/etc/machine-id` (stable per install, world-readable by design — this
/// binds, it does not hide).
pub struct SoftwareHsm {
    machine_id: Vec<u8>,
}

impl SoftwareHsm {
    pub fn new() -> Self {
        let machine_id = std::fs::read("/etc/machine-id")
            .ok()
            .map(|b| trim_ascii_compat(&b))
            .unwrap_or_else(|| b"unknown-machine".to_vec());
        Self { machine_id }
    }
}

/// `u8::trim_ascii` is 1.80+; same semantics on the 1.75 MSRV.
fn trim_ascii_compat(b: &[u8]) -> Vec<u8> {
    let mut s = b;
    while s.first().is_some_and(|c| c.is_ascii_whitespace()) {
        s = &s[1..];
    }
    while s.last().is_some_and(|c| c.is_ascii_whitespace()) {
        s = &s[..s.len() - 1];
    }
    s.to_vec()
}

impl Default for SoftwareHsm {
    fn default() -> Self {
        Self::new()
    }
}

impl HsmBackend for SoftwareHsm {
    fn name(&self) -> &'static str {
        "software-ram (Tier 1)"
    }

    fn isolates_keys(&self) -> bool {
        false
    }

    fn bind_local(&self, secret: &[u8], context: &[u8]) -> [u8; 48] {
        use sha2::{Digest, Sha384};
        let mut h = Sha384::new();
        h.update(b"Null-v2.0-hsm-bind:");
        h.update(context);
        h.update(&self.machine_id);
        h.update(secret);
        h.finalize().into()
    }
}

/// Tier 2 probe: TPM 2.0 resource manager, Apple Secure Enclave, YubiKey.
/// Present-day detection only — key operations through these devices need
/// their native stacks (`tpm2-tss`, `ykman`); until linked, attempts to
/// bind report exactly that instead of silently downgrading.
pub struct HardwareHsmProbe;

impl HardwareHsmProbe {
    /// Names of usable hardware isolates found on this machine (may be empty).
    pub fn detect() -> Vec<&'static str> {
        let mut found = Vec::new();
        if std::path::Path::new("/dev/tpmrm0").exists()
            || std::path::Path::new("/dev/tpm0").exists()
        {
            found.push("tpm2");
        }
        if std::path::Path::new("/usr/bin/ykman").exists()
            || std::path::Path::new("/usr/bin/ykchalresp").exists()
        {
            found.push("yubikey-slot2-hmac");
        }
        #[cfg(target_os = "macos")]
        found.push("secure-enclave");
        found
    }

    pub fn report() -> String {
        let found = Self::detect();
        if found.is_empty() {
            "no hardware isolate detected (Tier 1 software)".to_string()
        } else {
            format!(
                "hardware present [{}]; use native tools to seal identity keys",
                found.join(",")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_ascii_compat_trims_both_ends_only() {
        assert_eq!(trim_ascii_compat(b"  abc\n"), b"abc");
        assert_eq!(trim_ascii_compat(b"abc"), b"abc");
        assert_eq!(trim_ascii_compat(b" \t\r\n"), b"");
        assert_eq!(trim_ascii_compat(b""), b"");
        assert_eq!(trim_ascii_compat(b"a b"), b"a b");
    }

    #[test]
    fn hardened_roundtrip_and_wipe() {
        let mut b = HardenedBuffer::new(64).unwrap();
        b.as_mut_slice().copy_from_slice(&[7u8; 64]);
        assert_eq!(b.as_slice()[0], 7);
        b.panic_wipe();
        assert!(b.as_slice().iter().all(|&x| x == 0));
    }

    #[test]
    fn buffer_is_page_aligned_whole_pages() {
        let page = page_size();
        let b = HardenedBuffer::new(1).unwrap();
        assert_eq!(b.ptr as usize % page, 0, "page aligned");
        assert_eq!(b.layout.size(), page, "rounded up to whole pages");
        let c = HardenedBuffer::new(page + 1).unwrap();
        assert_eq!(c.layout.size(), 2 * page);
    }

    #[test]
    fn wipe_zeroes_padding_too() {
        let mut b = HardenedBuffer::new(8).unwrap();
        // SAFETY: test-only write over the full page the buffer owns.
        unsafe { std::ptr::write_bytes(b.ptr, 0xAA, b.layout.size()) };
        b.panic_wipe();
        let all = unsafe { std::slice::from_raw_parts(b.ptr, b.layout.size()) };
        assert!(all.iter().all(|&x| x == 0));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn harden_process_disables_dumps_and_core() {
        let r = harden_process();
        assert!(r.is_complete(), "{r:?}");
        let d = unsafe { os::prctl(os::PR_GET_DUMPABLE as _, 0, 0, 0, 0) };
        assert_eq!(d, 0);
        let mut lim = os::rlimit {
            rlim_cur: 1,
            rlim_max: 1,
        };
        assert_eq!(unsafe { os::getrlimit(os::RLIMIT_CORE, &mut lim) }, 0);
        assert_eq!((lim.rlim_cur, lim.rlim_max), (0, 0));
    }

    #[test]
    fn zero_len_rejected() {
        assert!(HardenedBuffer::new(0).is_err());
    }

    #[test]
    fn software_hsm_binds_deterministically() {
        let hsm = SoftwareHsm::new();
        assert_eq!(hsm.name(), "software-ram (Tier 1)");
        assert!(!hsm.isolates_keys());
        let a = hsm.bind_local(b"seed", b"ctx");
        let b = hsm.bind_local(b"seed", b"ctx");
        assert_eq!(a, b);
        assert_ne!(a, hsm.bind_local(b"other", b"ctx"));
        assert_ne!(a, hsm.bind_local(b"seed", b"other-ctx"));
    }

    #[test]
    fn hardware_probe_reports() {
        // Must not panic on any machine; content is environment-dependent.
        let _ = HardwareHsmProbe::report();
    }
}
