pub(crate) use vac_skills::install_system_skills;
pub(crate) use vac_skills::system_cache_root_dir;

use vac_utils_absolute_path::AbsolutePathBuf;

pub(crate) fn uninstall_system_skills(vac_home: &AbsolutePathBuf) {
    let _ = std::fs::remove_dir_all(system_cache_root_dir(vac_home));
}
