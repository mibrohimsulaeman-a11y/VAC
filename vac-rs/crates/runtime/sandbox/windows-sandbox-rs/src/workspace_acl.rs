use crate::acl::add_deny_write_ace;
use crate::path_normalization::canonicalize_path;
use anyhow::Result;
use std::ffi::c_void;
use std::path::Path;

pub fn is_command_cwd_root(root: &Path, canonical_command_cwd: &Path) -> bool {
    canonicalize_path(root) == canonical_command_cwd
}

/// # Safety
/// Caller must ensure `psid` is a valid SID pointer.
// SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; unsafe function contract for `protect_workspace_vac_dir` is documented by the surrounding wrapper and must be upheld by callers.
pub unsafe fn protect_workspace_vac_dir(cwd: &Path, psid: *mut c_void) -> Result<bool> {
    protect_workspace_subdir(cwd, psid, ".vac")
}

/// # Safety
/// Caller must ensure `psid` is a valid SID pointer.
// SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; unsafe function contract for `protect_workspace_agents_dir` is documented by the surrounding wrapper and must be upheld by callers.
pub unsafe fn protect_workspace_agents_dir(cwd: &Path, psid: *mut c_void) -> Result<bool> {
    protect_workspace_subdir(cwd, psid, ".agents")
}

// SAFETY: Win32 sandbox boundary: adjacent checked API calls create or validate the raw handle/SID/ACL/token/pipe pointer used here; unsafe function contract for `protect_workspace_subdir` is documented by the surrounding wrapper and must be upheld by callers.
unsafe fn protect_workspace_subdir(cwd: &Path, psid: *mut c_void, subdir: &str) -> Result<bool> {
    let path = cwd.join(subdir);
    if path.is_dir() {
        add_deny_write_ace(&path, psid)
    } else {
        Ok(false)
    }
}
