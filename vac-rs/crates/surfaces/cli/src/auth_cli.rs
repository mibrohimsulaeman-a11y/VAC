use std::io;
use std::io::Write;

use anyhow::Context;
use anyhow::bail;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use toml_edit::value;
use vac_core::config::edit;
use vac_core::config::edit::ConfigEdit;
use vac_core::config::find_vac_home;
use vac_protocol::config_types::ForcedLoginMethod;

const KILO_PROVIDER_ID: &str = "kilo";
const KILO_PROVIDER_NAME: &str = "Kilo Gateway";
const KILO_BASE_URL: &str = "https://api.kilo.ai/api/gateway";
const KILO_ENV_KEY: &str = "KILO_API_KEY";

#[derive(Debug, Parser)]
pub struct AuthCommand {
    #[command(subcommand)]
    command: AuthSubcommand,
}

impl AuthCommand {
    pub async fn run(self) -> anyhow::Result<AuthAction> {
        match self.command {
            AuthSubcommand::Login(command) => command.run().await,
        }
    }
}

#[derive(Debug)]
pub enum AuthAction {
    LaunchExistingLogin {
        forced_login_method: ForcedLoginMethod,
    },
    ConfiguredProvider,
}

#[derive(Debug, Subcommand)]
enum AuthSubcommand {
    /// Sign in or configure a model provider for the root VAC TUI/CLI path.
    Login(AuthLoginCommand),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum LoginProvider {
    /// Use the existing Vastar API-key onboarding path.
    VastarApiKey,
    /// Configure Kilo Gateway as a custom model provider.
    Kilo,
    /// Configure a user-defined provider entry in ~/.vac/config.toml.
    Custom,
}

#[derive(Debug, Parser)]
struct AuthLoginCommand {
    /// Provider/auth method. Omit for an interactive chooser.
    #[arg(long, value_enum)]
    provider: Option<LoginProvider>,

    /// Provider id written to `model_provider` and `[model_providers.<id>]` for Kilo/custom.
    #[arg(long = "provider-id")]
    provider_id: Option<String>,

    /// Friendly provider display name for Kilo/custom.
    #[arg(long = "name")]
    name: Option<String>,

    /// Provider base URL for Kilo/custom.
    #[arg(long = "base-url")]
    base_url: Option<String>,

    /// Environment variable used for the provider API key for Kilo/custom.
    #[arg(long = "api-key-env")]
    api_key_env: Option<String>,

    /// Default model to write to `model` for Kilo/custom. If omitted, the existing model is preserved.
    #[arg(long = "model")]
    model: Option<String>,

    /// Store the entered Kilo/custom API key in `~/.vac/config.toml` as `experimental_bearer_token`.
    ///
    /// This is convenient for local dogfood, but it stores the secret in plaintext.
    #[arg(long = "store-api-key-in-config", default_value_t = false)]
    store_api_key_in_config: bool,

    /// Do not prompt before writing config. Requires --provider for non-interactive use.
    #[arg(long = "yes", short = 'y', default_value_t = false)]
    yes: bool,
}

impl AuthLoginCommand {
    async fn run(self) -> anyhow::Result<AuthAction> {
        let preset = match self.provider {
            Some(provider) => provider,
            None => prompt_provider()?,
        };

        match preset {
            LoginProvider::VastarApiKey => Ok(AuthAction::LaunchExistingLogin {
                forced_login_method: ForcedLoginMethod::Api,
            }),
            LoginProvider::Kilo | LoginProvider::Custom => {
                self.configure_custom_provider(preset)?;
                Ok(AuthAction::ConfiguredProvider)
            }
        }
    }

    fn configure_custom_provider(self, preset: LoginProvider) -> anyhow::Result<()> {
        let spec = ProviderSpec::from_command(preset, self)?;
        spec.validate()?;

        if !spec.assume_yes {
            print_summary(&spec);
            if !prompt_yes_no("Write this provider config now?", true)? {
                println!("Cancelled. No config changes were written.");
                return Ok(());
            }
        }

        let mut credential = CredentialConfig::EnvKey {
            env_key: spec.api_key_env.clone(),
        };

        if spec.store_api_key_in_config || (!spec.assume_yes && prompt_store_key_in_config()?) {
            println!("API key input is visible in this terminal session.");
            let api_key = prompt_required("API key")?;
            credential = CredentialConfig::PlaintextBearer { api_key };
        }

        let vac_home = find_vac_home()?;
        std::fs::create_dir_all(&vac_home)
            .with_context(|| format!("failed to create VAC home at {}", vac_home.display()))?;

        edit::apply_blocking(&vac_home, None, &build_edits(&spec, &credential)?)?;

        println!(
            "Configured provider `{}` in {}",
            spec.provider_id,
            vac_home.join("config.toml").display()
        );
        match credential {
            CredentialConfig::EnvKey { env_key } => println!(
                "Set {env_key} in your shell before starting VAC, for example: export {env_key}=<your_api_key>"
            ),
            CredentialConfig::PlaintextBearer { .. } => println!(
                "API key stored in config as experimental_bearer_token. Treat ~/.vac/config.toml as secret material."
            ),
        }
        println!("Start the TUI with: vac");

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct ProviderSpec {
    provider_id: String,
    name: String,
    base_url: String,
    api_key_env: String,
    model: Option<String>,
    store_api_key_in_config: bool,
    assume_yes: bool,
}

impl ProviderSpec {
    fn from_command(provider: LoginProvider, command: AuthLoginCommand) -> anyhow::Result<Self> {
        let defaults = ProviderDefaults::for_provider(provider);
        let provider_id = match command.provider_id {
            Some(value) => value,
            None if provider == LoginProvider::Custom && !command.yes => {
                prompt_required("Provider id")?
            }
            None => defaults.provider_id.to_string(),
        };
        let name = match command.name {
            Some(value) => value,
            None if provider == LoginProvider::Custom && !command.yes => {
                prompt_required("Display name")?
            }
            None => defaults.name.to_string(),
        };
        let base_url = match command.base_url {
            Some(value) => value,
            None if provider == LoginProvider::Custom && !command.yes => {
                prompt_required("Base URL")?
            }
            None => defaults.base_url.to_string(),
        };
        let api_key_env = match command.api_key_env {
            Some(value) => value,
            None if provider == LoginProvider::Custom && !command.yes => {
                prompt_with_default("API key environment variable", defaults.api_key_env)?
            }
            None => defaults.api_key_env.to_string(),
        };
        let model = match command.model {
            Some(value) if value.trim().is_empty() => None,
            Some(value) => Some(value),
            None if provider == LoginProvider::Custom && !command.yes => {
                prompt_optional("Default model (leave blank to preserve existing)")?
                    .filter(|model| !model.trim().is_empty())
            }
            None => None,
        };

        Ok(Self {
            provider_id,
            name,
            base_url,
            api_key_env,
            model,
            store_api_key_in_config: command.store_api_key_in_config,
            assume_yes: command.yes,
        })
    }

    fn validate(&self) -> anyhow::Result<()> {
        validate_provider_id(&self.provider_id)?;
        validate_non_empty("name", &self.name)?;
        validate_non_empty("base-url", &self.base_url)?;
        validate_non_empty("api-key-env", &self.api_key_env)?;
        if !self.base_url.starts_with("https://") && !self.base_url.starts_with("http://localhost")
        {
            bail!("base-url must use https://, except localhost is allowed for local testing");
        }
        Ok(())
    }
}

struct ProviderDefaults {
    provider_id: &'static str,
    name: &'static str,
    base_url: &'static str,
    api_key_env: &'static str,
}

impl ProviderDefaults {
    fn for_provider(provider: LoginProvider) -> Self {
        match provider {
            LoginProvider::Kilo => Self {
                provider_id: KILO_PROVIDER_ID,
                name: KILO_PROVIDER_NAME,
                base_url: KILO_BASE_URL,
                api_key_env: KILO_ENV_KEY,
            },
            LoginProvider::Custom => Self {
                provider_id: "custom",
                name: "Custom Provider",
                base_url: "https://example.com/v1",
                api_key_env: "CUSTOM_API_KEY",
            },
            LoginProvider::VastarApiKey => {
                unreachable!("built-in auth provider does not use provider registry defaults")
            }
        }
    }
}

enum CredentialConfig {
    EnvKey { env_key: String },
    PlaintextBearer { api_key: String },
}

fn build_edits(
    spec: &ProviderSpec,
    credential: &CredentialConfig,
) -> anyhow::Result<Vec<ConfigEdit>> {
    let provider_path = |key: &str| {
        vec![
            "model_providers".to_string(),
            spec.provider_id.clone(),
            key.to_string(),
        ]
    };

    let mut edits = vec![
        ConfigEdit::SetPath {
            segments: vec!["model_provider".to_string()],
            value: value(spec.provider_id.clone()),
        },
        ConfigEdit::SetPath {
            segments: provider_path("name"),
            value: value(spec.name.clone()),
        },
        ConfigEdit::SetPath {
            segments: provider_path("base_url"),
            value: value(spec.base_url.clone()),
        },
        ConfigEdit::SetPath {
            segments: provider_path("wire_api"),
            value: value("responses"),
        },
        ConfigEdit::SetPath {
            segments: provider_path("requires_vastar_auth"),
            value: value(false),
        },
    ];

    if let Some(model) = spec.model.as_ref() {
        edits.push(ConfigEdit::SetPath {
            segments: vec!["model".to_string()],
            value: value(model.clone()),
        });
    }

    match credential {
        CredentialConfig::EnvKey { env_key } => {
            edits.push(ConfigEdit::SetPath {
                segments: provider_path("env_key"),
                value: value(env_key.clone()),
            });
            edits.push(ConfigEdit::SetPath {
                segments: provider_path("env_key_instructions"),
                value: value(format!(
                    "Set {env_key} to your provider API key before starting VAC."
                )),
            });
            edits.push(ConfigEdit::ClearPath {
                segments: provider_path("experimental_bearer_token"),
            });
        }
        CredentialConfig::PlaintextBearer { api_key } => {
            validate_non_empty("api-key", api_key)?;
            edits.push(ConfigEdit::SetPath {
                segments: provider_path("experimental_bearer_token"),
                value: value(api_key.clone()),
            });
            edits.push(ConfigEdit::ClearPath {
                segments: provider_path("env_key"),
            });
            edits.push(ConfigEdit::ClearPath {
                segments: provider_path("env_key_instructions"),
            });
        }
    }

    Ok(edits)
}

fn prompt_provider() -> anyhow::Result<LoginProvider> {
    println!("Select auth/provider path:");
    println!("  1. Vastar API key (local onboarding)");
    println!("  2. Kilo Gateway provider");
    println!("  3. Custom provider");
    loop {
        let input = prompt_required("Provider [1-3]")?;
        match input.trim() {
            "1" | "api" | "vastar" | "vastar-api-key" => return Ok(LoginProvider::VastarApiKey),
            "2" | "kilo" | "Kilo" => return Ok(LoginProvider::Kilo),
            "3" | "custom" | "Custom" => return Ok(LoginProvider::Custom),
            _ => println!("Enter 1, 2, or 3."),
        }
    }
}

fn print_summary(spec: &ProviderSpec) {
    println!("\nProvider config:");
    println!("  provider id: {}", spec.provider_id);
    println!("  name:        {}", spec.name);
    println!("  base URL:    {}", spec.base_url);
    println!("  wire API:    responses");
    match spec.model.as_ref() {
        Some(model) => println!("  model:       {model}"),
        None => println!("  model:       preserve existing config"),
    }
    if spec.store_api_key_in_config {
        println!("  credential:  store API key in config plaintext");
    } else {
        println!("  credential:  environment variable {}", spec.api_key_env);
    }
    println!();
}

fn prompt_store_key_in_config() -> anyhow::Result<bool> {
    println!("Credential storage:");
    println!("  Recommended: keep the API key in an environment variable.");
    println!("  Convenience: store the API key in ~/.vac/config.toml as plaintext.");
    prompt_yes_no("Store API key in config plaintext?", false)
}

fn prompt_yes_no(prompt: &str, default: bool) -> anyhow::Result<bool> {
    let suffix = if default { "[Y/n]" } else { "[y/N]" };
    loop {
        print!("{prompt} {suffix}: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let answer = input.trim();
        if answer.is_empty() {
            return Ok(default);
        }
        if answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes") {
            return Ok(true);
        }
        if answer.eq_ignore_ascii_case("n") || answer.eq_ignore_ascii_case("no") {
            return Ok(false);
        }
    }
}

fn prompt_required(prompt: &str) -> anyhow::Result<String> {
    loop {
        if let Some(value) = prompt_optional(prompt)?
            && !value.trim().is_empty()
        {
            return Ok(value.trim().to_string());
        }
        println!("{prompt} is required.");
    }
}

fn prompt_with_default(prompt: &str, default: &str) -> anyhow::Result<String> {
    print!("{prompt} [{default}]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let value = input.trim();
    if value.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(value.to_string())
    }
}

fn prompt_optional(prompt: &str) -> anyhow::Result<Option<String>> {
    print!("{prompt}: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(Some(input.trim().to_string()))
}

fn validate_provider_id(value: &str) -> anyhow::Result<()> {
    validate_non_empty("provider-id", value)?;
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        bail!("provider-id may only contain ASCII letters, numbers, '-' and '_'");
    }
    Ok(())
}

fn validate_non_empty(label: &str, value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        bail!("{label} must not be empty");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kilo_defaults_are_stable() {
        let defaults = ProviderDefaults::for_provider(LoginProvider::Kilo);
        assert_eq!(defaults.provider_id, "kilo");
        assert_eq!(defaults.name, "Kilo Gateway");
        assert_eq!(defaults.base_url, KILO_BASE_URL);
        assert_eq!(defaults.api_key_env, "KILO_API_KEY");
    }

    #[test]
    fn provider_id_rejects_dotted_paths() {
        assert!(validate_provider_id("kilo").is_ok());
        assert!(validate_provider_id("kilo.gateway").is_err());
    }

    #[test]
    fn env_key_edits_do_not_store_plaintext_secret() {
        let spec = ProviderSpec {
            provider_id: "kilo".to_string(),
            name: "Kilo Gateway".to_string(),
            base_url: KILO_BASE_URL.to_string(),
            api_key_env: "KILO_API_KEY".to_string(),
            model: Some("anthropic/claude-sonnet-4.5".to_string()),
            store_api_key_in_config: false,
            assume_yes: true,
        };
        let edits = build_edits(
            &spec,
            &CredentialConfig::EnvKey {
                env_key: "KILO_API_KEY".to_string(),
            },
        )
        .expect("edits");
        assert!(edits.iter().any(|edit| {
            matches!(edit, ConfigEdit::SetPath { segments, .. } if segments == &vec!["model_provider".to_string()])
        }));
        assert!(edits.iter().any(|edit| {
            matches!(edit, ConfigEdit::ClearPath { segments } if segments.last().is_some_and(|segment| segment == "experimental_bearer_token"))
        }));
    }
}
