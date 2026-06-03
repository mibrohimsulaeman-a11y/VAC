use anyhow::anyhow;
use anyhow::Result;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Foundation::HLOCAL;
use windows_sys::Win32::Security::Cryptography::CryptProtectData;
use windows_sys::Win32::Security::Cryptography::CryptUnprotectData;
use windows_sys::Win32::Security::Cryptography::CRYPTPROTECT_LOCAL_MACHINE;
use windows_sys::Win32::Security::Cryptography::CRYPTPROTECT_UI_FORBIDDEN;
use windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB;

fn make_blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    }
}

#[allow(clippy::unnecessary_mut_passed)]
pub fn protect(data: &[u8]) -> Result<Vec<u8>> {
    let mut in_blob = make_blob(data);
    let mut out_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; the multi-line unsafe block below uses inputs that remain live until the block returns.
    let ok = unsafe {
        CryptProtectData(
            &mut in_blob,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            // Use machine scope so elevated and non-elevated processes can decrypt.
            CRYPTPROTECT_UI_FORBIDDEN | CRYPTPROTECT_LOCAL_MACHINE,
            &mut out_blob,
        )
    };
    if ok == 0 {
        // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; the multi-line unsafe block below uses inputs that remain live until the block returns.
        return Err(anyhow!("CryptProtectData failed: {}", unsafe {
            GetLastError()
        }));
    }
    let slice =
        // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; `std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize)` uses inputs that remain live for the duration of this block.
        unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) }.to_vec();
    // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; the multi-line unsafe block below uses inputs that remain live until the block returns.
    unsafe {
        if !out_blob.pbData.is_null() {
            LocalFree(out_blob.pbData as HLOCAL);
        }
    }
    Ok(slice)
}

#[allow(clippy::unnecessary_mut_passed)]
pub fn unprotect(blob: &[u8]) -> Result<Vec<u8>> {
    let mut in_blob = make_blob(blob);
    let mut out_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; the multi-line unsafe block below uses inputs that remain live until the block returns.
    let ok = unsafe {
        CryptUnprotectData(
            &mut in_blob,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            // Use machine scope so elevated and non-elevated processes can decrypt.
            CRYPTPROTECT_UI_FORBIDDEN | CRYPTPROTECT_LOCAL_MACHINE,
            &mut out_blob,
        )
    };
    if ok == 0 {
        // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; the multi-line unsafe block below uses inputs that remain live until the block returns.
        return Err(anyhow!("CryptUnprotectData failed: {}", unsafe {
            GetLastError()
        }));
    }
    let slice =
        // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; `std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize)` uses inputs that remain live for the duration of this block.
        unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) }.to_vec();
    // SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; the multi-line unsafe block below uses inputs that remain live until the block returns.
    unsafe {
        if !out_blob.pbData.is_null() {
            LocalFree(out_blob.pbData as HLOCAL);
        }
    }
    Ok(slice)
}
