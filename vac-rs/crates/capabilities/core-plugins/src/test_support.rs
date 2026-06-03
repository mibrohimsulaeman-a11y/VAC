use std::fs;
use std::path::Path;

use crate::PluginsConfigInput;
use crate::VASTAR_CURATED_MARKETPLACE_NAME;
use toml::Value;
use vac_config::LoaderOverrides;
use vac_config::NoopThreadConfigLoader;
use vac_config::loader::load_config_layers_state;
use vac_exec_server::LOCAL_FS;
use vac_utils_absolute_path::AbsolutePathBuf;

pub(crate) const TEST_CURATED_PLUGIN_SHA: &str = "0123456789abcdef0123456789abcdef01234567";
pub(crate) const TEST_CURATED_PLUGIN_CACHE_VERSION: &str = "01234567";

pub(crate) fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("file should have a parent")).unwrap();
    fs::write(path, contents).unwrap();
}

pub(crate) fn write_curated_plugin(root: &Path, plugin_name: &str) {
    let plugin_root = root.join("plugins").join(plugin_name);
    write_file(
        &plugin_root.join(".vac-plugin/plugin.json"),
        &format!(
            r#"{{
  "name": "{plugin_name}",
  "description": "Plugin that includes skills, MCP servers, and app connectors"
}}"#
        ),
    );
    write_file(
        &plugin_root.join("skills/SKILL.md"),
        "---\nname: sample\ndescription: sample\n---\n",
    );
    write_file(
        &plugin_root.join(".mcp.json"),
        r#"{
  "mcpServers": {
    "sample-docs": {
      "type": "http",
      "url": "https://sample.example/mcp"
    }
  }
}"#,
    );
    write_file(
        &plugin_root.join(".app.json"),
        r#"{
  "apps": {
    "calendar": {
      "id": "connector_calendar"
    }
  }
}"#,
    );
}

pub(crate) fn write_vastar_curated_marketplace(root: &Path, plugin_names: &[&str]) {
    let plugins = plugin_names
        .iter()
        .map(|plugin_name| {
            format!(
                r#"{{
      "name": "{plugin_name}",
      "source": {{
        "source": "local",
        "path": "./plugins/{plugin_name}"
      }}
    }}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    write_file(
        &root.join(".agents/plugins/marketplace.json"),
        &format!(
            r#"{{
  "name": "{VASTAR_CURATED_MARKETPLACE_NAME}",
  "plugins": [
{plugins}
  ]
}}"#
        ),
    );
    for plugin_name in plugin_names {
        write_curated_plugin(root, plugin_name);
    }
}

pub(crate) fn write_curated_plugin_sha(vac_home: &Path) {
    write_curated_plugin_sha_with(vac_home, TEST_CURATED_PLUGIN_SHA);
}

pub(crate) fn write_curated_plugin_sha_with(vac_home: &Path, sha: &str) {
    write_file(&vac_home.join(".tmp/plugins.sha"), &format!("{sha}\n"));
}

pub(crate) async fn load_plugins_config(vac_home: &Path, cwd: &Path) -> PluginsConfigInput {
    let vac_home = AbsolutePathBuf::try_from(vac_home).expect("vac home should be absolute");
    let cwd = AbsolutePathBuf::try_from(cwd).expect("cwd should be absolute");
    let config_layer_stack = load_config_layers_state(
        LOCAL_FS.as_ref(),
        vac_home.as_path(),
        Some(cwd),
        &[],
        LoaderOverrides::without_managed_config_for_tests(),
        &NoopThreadConfigLoader,
    )
    .await
    .expect("config should load");
    let effective_config = config_layer_stack.effective_config();
    PluginsConfigInput::new(
        config_layer_stack,
        feature_enabled(&effective_config, "plugins", /*default_enabled*/ true),
        feature_enabled(
            &effective_config,
            "remote_plugin",
            /*default_enabled*/ false,
        ),
        feature_enabled(
            &effective_config,
            "plugin_hooks",
            /*default_enabled*/ false,
        ),
        "https://provider-plugins.local/disabled/".to_string(),
    )
}

fn feature_enabled(config: &Value, key: &str, default_enabled: bool) -> bool {
    config
        .get("features")
        .and_then(Value::as_table)
        .and_then(|features| features.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(default_enabled)
}
