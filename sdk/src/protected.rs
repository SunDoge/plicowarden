//! Protected signing buffer, copied from Warden Desktop (GPL-3.0-only).
use std::ptr::NonNull;
use zeroize::{Zeroize, Zeroizing};

#[derive(Debug, snafu::Snafu)]
pub enum SecretMemoryError {
    #[snafu(display("Invalid protected secret size"))]
    Size,
    #[snafu(display("Could not allocate protected secret memory"))]
    Allocation,
    #[snafu(display("Could not lock secret memory"))]
    Lock,
    #[snafu(display("Could not exclude secret memory from core dumps"))]
    Dump,
    #[snafu(display("Could not make secret memory read-only"))]
    ReadOnly,
}
/// Bounded, non-cloneable secret bytes, protected by memsec guard pages/canary.
/// Locking and read-only protection are checked, not merely requested. Linux
/// additionally checks MADV_DONTDUMP on the allocation's data pages.
/// No Debug/Display/serde implementations, and no ownership escape for the bytes.
pub struct LockedSecret {
    ptr: NonNull<[u8]>,
}
impl LockedSecret {
    pub fn from_string(mut value: Zeroizing<String>) -> Result<Self, SecretMemoryError> {
        if value.is_empty() || value.len() > 1024 * 1024 {
            return Err(SecretMemoryError::Size);
        }
        // SAFETY: memsec owns allocation geometry and guard pages. The returned
        // allocation is uniquely owned by this value and always freed by Drop.
        let ptr =
            unsafe { memsec::malloc_sized(value.len()) }.ok_or(SecretMemoryError::Allocation)?;
        let secret = Self { ptr };
        // memsec's allocator only attempts mlock; explicitly check it here.
        // SAFETY: ptr covers value.len() live, writable, uniquely owned bytes.
        if !unsafe { memsec::mlock(secret.ptr.as_ptr() as *mut u8, value.len()) } {
            return Err(SecretMemoryError::Lock);
        }
        #[cfg(target_os = "linux")]
        {
            let page = rustix::param::page_size();
            let address = secret.ptr.as_ptr() as *mut u8 as usize;
            let base = address / page * page;
            let size = (address - base + value.len()).div_ceil(page) * page;
            // SAFETY: memsec allocates dedicated pages surrounding these bytes;
            // the aligned range contains only its live data/canary pages, not
            // either inaccessible guard page or a shared allocator arena.
            unsafe { rustix::mm::madvise(base as *mut _, size, rustix::mm::Advice::LinuxDontDump) }
                .map_err(|_| SecretMemoryError::Dump)?;
        }
        // SAFETY: source and destination are separate, valid allocations of the
        // same length; destination is writable until the protection below.
        unsafe {
            std::ptr::copy_nonoverlapping(
                value.as_ptr(),
                secret.ptr.as_ptr() as *mut u8,
                value.len(),
            );
        }
        value.zeroize();
        // SAFETY: allocation is exclusively owned; this changes only memsec's
        // data pages. free() restores writable access before wiping and unmapping.
        if !unsafe { memsec::mprotect(secret.ptr, memsec::Prot::ReadOnly) } {
            return Err(SecretMemoryError::ReadOnly);
        }
        Ok(secret)
    }
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: the allocation is live for self's lifetime and immutable.
        unsafe { self.ptr.as_ref() }
    }
}
impl Drop for LockedSecret {
    fn drop(&mut self) {
        // SAFETY: this is the original memsec allocation and has never been
        // freed or exposed for mutation. free restores protection, validates the
        // canary, wipes the data pages, unlocks them, then releases the mapping.
        unsafe {
            memsec::free(self.ptr);
        }
    }
}
