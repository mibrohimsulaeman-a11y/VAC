use anyhow::Result;
use std::ffi::OsStr;
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Foundation::ERROR_ALREADY_EXISTS;
use windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::System::Threading::MUTEX_ALL_ACCESS;
use windows_sys::Win32::System::Threading::OpenMutexW;
use windows_sys::Win32::System::Threading::ReleaseMutex;

use super::to_wide;

const READ_ACL_MUTEX_NAME: &str = "Local\\VACSandboxReadAcl";

pub struct ReadAclMutexGuard {
    handle: HANDLE,
}

impl Drop for ReadAclMutexGuard {
    fn drop(&mut self) {
        // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; the multi-line unsafe block below uses inputs that remain live until the block returns.
        unsafe {
            let _ = ReleaseMutex(self.handle);
            CloseHandle(self.handle);
        }
    }
}

pub fn read_acl_mutex_exists() -> Result<bool> {
    let name = to_wide(OsStr::new(READ_ACL_MUTEX_NAME));
    // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; `OpenMutexW(MUTEX_ALL_ACCESS, 0, name.as_ptr())` uses inputs that remain live for the duration of this block.
    let handle = unsafe { OpenMutexW(MUTEX_ALL_ACCESS, 0, name.as_ptr()) };
    if handle == 0 {
        // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; `GetLastError()` uses inputs that remain live for the duration of this block.
        let err = unsafe { GetLastError() };
        if err == ERROR_FILE_NOT_FOUND {
            return Ok(false);
        }
        return Err(anyhow::anyhow!("OpenMutexW failed: {err}"));
    }
    // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; the multi-line unsafe block below uses inputs that remain live until the block returns.
    unsafe {
        CloseHandle(handle);
    }
    Ok(true)
}

pub fn acquire_read_acl_mutex() -> Result<Option<ReadAclMutexGuard>> {
    let name = to_wide(OsStr::new(READ_ACL_MUTEX_NAME));
    // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; `CreateMutexW(std::ptr::null_mut(), 1, name.as_ptr())` uses inputs that remain live for the duration of this block.
    let handle = unsafe { CreateMutexW(std::ptr::null_mut(), 1, name.as_ptr()) };
    if handle == 0 {
        // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; the multi-line unsafe block below uses inputs that remain live until the block returns.
        return Err(anyhow::anyhow!("CreateMutexW failed: {}", unsafe {
            GetLastError()
        }));
    }
    // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; `GetLastError()` uses inputs that remain live for the duration of this block.
    let err = unsafe { GetLastError() };
    if err == ERROR_ALREADY_EXISTS {
        // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; the multi-line unsafe block below uses inputs that remain live until the block returns.
        unsafe {
            CloseHandle(handle);
        }
        return Ok(None);
    }
    Ok(Some(ReadAclMutexGuard { handle }))
}
