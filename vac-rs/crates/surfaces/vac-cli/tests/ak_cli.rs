use std::process::Command;

const RETROSPECT_MARKDOWN: &str =
    include_str!("../../../capabilities/vac-ak/src/skills/retrospect.v1.md");

const AUTOPILOT_ONE_LINER: &str = r#"vac autopilot schedule add --name retrospect --cron "0 3 * * *" --prompt "$(vac ak skill retrospect)""#;

#[test]
fn ak_skill_retrospect_prints_bundled_prompt() {
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let home = temp_dir.path();

    let output = Command::new(env!("CARGO_BIN_EXE_vac"))
        .arg("ak")
        .arg("skill")
        .arg("retrospect")
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("VAC_PROFILE")
        .output()
        .expect("run vac ak skill retrospect");

    assert!(
        output.status.success(),
        "ak skill retrospect failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    // `println!` appends a trailing newline after the content; the bundled
    // file may or may not end with one, so compare on a trim-end basis.
    assert_eq!(
        stdout.trim_end_matches('\n'),
        RETROSPECT_MARKDOWN.trim_end_matches('\n'),
        "vac ak skill retrospect output does not match bundled retrospect.v1.md"
    );
}

#[test]
fn readme_autopilot_one_liner_matches_retrospect_prompt() {
    // Prevent drift between the autopilot docs and the one-liner embedded
    // in the skill prompt itself.
    assert!(
        RETROSPECT_MARKDOWN.contains(AUTOPILOT_ONE_LINER),
        "bundled retrospect.v1.md must contain the canonical autopilot one-liner"
    );

    let root_readme = include_str!("../../README.md");
    let cli_readme = include_str!("../README.md");
    assert!(
        root_readme.contains(AUTOPILOT_ONE_LINER) || cli_readme.contains(AUTOPILOT_ONE_LINER),
        "the canonical `autopilot schedule add` one-liner must appear in the autopilot docs (README.md or cli/README.md) so the docs and SKILL_RETROSPECT stay in sync"
    );
}

#[cfg(windows)]
#[test]
fn windows_spawned_vac_ak_skill_does_not_stack_overflow() {
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let home = temp_dir.path();

    let output = Command::new(env!("CARGO_BIN_EXE_vac"))
        .arg("ak")
        .arg("skill")
        .arg("retrospect")
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("VAC_PROFILE")
        .output()
        .expect("spawn vac ak skill retrospect on Windows");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "spawned vac ak skill retrospect failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr
    );
    assert!(
        !stderr.contains("overflowed its stack"),
        "spawned vac.exe must not stack overflow on Windows"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim_end_matches('\n'),
        RETROSPECT_MARKDOWN.trim_end_matches('\n')
    );
}

#[test]
fn ak_search_tree_bootstraps_default_config_on_clean_home() {
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let home = temp_dir.path();

    let output = Command::new(env!("CARGO_BIN_EXE_vac"))
        .arg("ak")
        .arg("search")
        .arg("--tree")
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("VAC_PROFILE")
        .output()
        .expect("run vac ak search --tree");

    assert!(
        output.status.success(),
        "ak search --tree failed: stdout={} stderr= {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("."), "stdout was: {stdout}");

    assert!(
        home.join(".vac/config.toml").is_file(),
        "expected ak search --tree to bootstrap ~/.vac/config.toml"
    );
}

#[test]
fn ak_search_rejects_tree_with_grep() {
    let output = Command::new(env!("CARGO_BIN_EXE_vac"))
        .arg("ak")
        .arg("search")
        .arg("--tree")
        .arg("--grep")
        .arg("foo")
        .output()
        .expect("run vac ak search with invalid flags");

    assert!(!output.status.success(), "command unexpectedly succeeded");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--tree"), "stderr was: {stderr}");
    assert!(stderr.contains("--grep"), "stderr was: {stderr}");
}

#[test]
fn ak_read_multiple_paths_uses_delimiter() {
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let home = temp_dir.path();
    let store = temp_dir.path().join("knowledge");
    std::fs::create_dir_all(&store).expect("create store");
    std::fs::write(store.join("a.md"), "alpha\n").expect("write first file");
    std::fs::write(store.join("b.md"), "beta\n").expect("write second file");

    let output = Command::new(env!("CARGO_BIN_EXE_vac"))
        .arg("ak")
        .arg("read")
        .arg("a.md")
        .arg("b.md")
        .env("AK_STORE", &store)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .output()
        .expect("run vac ak read");

    assert!(
        output.status.success(),
        "ak read failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "alpha\n---\nbeta\n"
    );
}

#[test]
fn ak_read_on_directory_returns_search_hint() {
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let home = temp_dir.path();
    let store = temp_dir.path().join("knowledge");
    std::fs::create_dir_all(store.join("services")).expect("create services dir");

    let output = Command::new(env!("CARGO_BIN_EXE_vac"))
        .arg("ak")
        .arg("read")
        .arg("services")
        .env("AK_STORE", &store)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("VAC_PROFILE")
        .output()
        .expect("run vac ak read on directory");

    assert!(
        !output.status.success(),
        "command unexpectedly succeeded: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("is a directory"), "stderr was: {stderr}");
    assert!(
        stderr.contains("ak search services"),
        "stderr was: {stderr}"
    );
}

#[test]
fn ak_remove_missing_path_returns_clear_error() {
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let home = temp_dir.path();
    let store = temp_dir.path().join("knowledge");
    std::fs::create_dir_all(&store).expect("create store");

    let output = Command::new(env!("CARGO_BIN_EXE_vac"))
        .arg("ak")
        .arg("remove")
        .arg("missing.md")
        .env("AK_STORE", &store)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("VAC_PROFILE")
        .output()
        .expect("run vac ak remove on missing path");

    assert!(
        !output.status.success(),
        "command unexpectedly succeeded: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("path not found: missing.md"),
        "stderr was: {stderr}"
    );
}
