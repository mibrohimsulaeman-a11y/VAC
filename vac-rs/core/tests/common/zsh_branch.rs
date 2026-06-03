use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use vac_core::config::Config;
use vac_core::config::Constrained;
use vac_features::Feature;
use vac_protocol::models::PermissionProfile;
use vac_protocol::permissions::NetworkSandboxPolicy;
use vac_protocol::protocol::AskForApproval;

use crate::test_vac::TestVAC;
use crate::test_vac::test_vac;

#[derive(Clone)]
pub struct ZshBranchRuntime {
    zsh_path: PathBuf,
    main_execve_wrapper_exe: PathBuf,
}

impl ZshBranchRuntime {
    fn apply_to_config(
        &self,
        config: &mut Config,
        approval_policy: AskForApproval,
        permission_profile: PermissionProfile,
    ) {
        config
            .features
            .enable(Feature::ShellTool)
            .expect("test config should allow feature update");
        config
            .features
            .enable(Feature::ShellZshBranch)
            .expect("test config should allow feature update");
        config.zsh_path = Some(self.zsh_path.clone());
        config.main_execve_wrapper_exe = Some(self.main_execve_wrapper_exe.clone());
        config.permissions.allow_login_shell = false;
        config.permissions.approval_policy = Constrained::allow_any(approval_policy);
        config
            .permissions
            .set_permission_profile(permission_profile)
            .expect("set permission profile");
    }
}

pub fn restrictive_workspace_write_profile() -> PermissionProfile {
    PermissionProfile::workspace_write_with(
        &[],
        NetworkSandboxPolicy::Restricted,
        /*exclude_tmpdir_env_var*/ true,
        /*exclude_slash_tmp*/ true,
    )
}

pub fn zsh_branch_runtime(test_name: &str) -> Result<Option<ZshBranchRuntime>> {
    let Some(zsh_path) = find_test_zsh_path()? else {
        return Ok(None);
    };
    if !supports_exec_wrapper_intercept(&zsh_path) {
        eprintln!(
            "skipping {test_name}: zsh does not support EXEC_WRAPPER intercepts ({})",
            zsh_path.display()
        );
        return Ok(None);
    }
    let Ok(main_execve_wrapper_exe) = vac_utils_cargo_bin::cargo_bin("vac-execve-wrapper") else {
        eprintln!("skipping {test_name}: unable to resolve `vac-execve-wrapper` binary");
        return Ok(None);
    };

    Ok(Some(ZshBranchRuntime {
        zsh_path,
        main_execve_wrapper_exe,
    }))
}

pub async fn build_zsh_branch_test<F>(
    server: &wiremock::MockServer,
    runtime: ZshBranchRuntime,
    approval_policy: AskForApproval,
    permission_profile: PermissionProfile,
    pre_build_hook: F,
) -> Result<TestVAC>
where
    F: FnOnce(&Path) + Send + 'static,
{
    let mut builder = test_vac()
        .with_pre_build_hook(pre_build_hook)
        .with_config(move |config| {
            runtime.apply_to_config(config, approval_policy, permission_profile);
        });
    builder.build(server).await
}

fn find_test_zsh_path() -> Result<Option<PathBuf>> {
    let repo_root = vac_utils_cargo_bin::repo_root()?;
    let dotslash_zsh = repo_root.join("vac-rs/app-server/tests/suite/zsh");
    if !dotslash_zsh.is_file() {
        eprintln!(
            "skipping zsh-branch test: shared zsh DotSlash file not found at {}",
            dotslash_zsh.display()
        );
        return Ok(None);
    }

    match crate::fetch_dotslash_file(&dotslash_zsh, /*dotslash_cache*/ None) {
        Ok(path) => Ok(Some(path)),
        Err(error) => {
            eprintln!("skipping zsh-branch test: failed to fetch zsh via dotslash: {error:#}");
            Ok(None)
        }
    }
}

fn supports_exec_wrapper_intercept(zsh_path: &Path) -> bool {
    let status = std::process::Command::new(zsh_path)
        .arg("-fc")
        .arg("/usr/bin/true")
        .env("EXEC_WRAPPER", "/usr/bin/false")
        .status();
    match status {
        Ok(status) => !status.success(),
        Err(_) => false,
    }
}
