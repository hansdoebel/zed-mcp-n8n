use schemars::JsonSchema;
use serde::Deserialize;
use std::env;
use zed_extension_api::{
    self as zed, settings::ContextServerSettings, Command, ContextServerConfiguration,
    ContextServerId, Project, Result,
};

const MCP_REMOTE_PACKAGE: &str = "mcp-remote";
const MCP_REMOTE_ENTRY: &str = "node_modules/mcp-remote/dist/proxy.js";
const SUPERGATEWAY_PACKAGE: &str = "supergateway";
const SUPERGATEWAY_ENTRY: &str = "node_modules/supergateway/dist/index.js";

#[derive(Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum AuthMode {
    #[default]
    Oauth,
    Token,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct N8nMcpSettings {
    server_url: String,
    #[serde(default)]
    auth: AuthMode,
    #[serde(default)]
    access_token: Option<String>,
}

#[derive(Debug)]
struct LaunchPlan {
    package: &'static str,
    entry: &'static str,
    trailing_args: Vec<String>,
}

fn resolve_launch(settings: &N8nMcpSettings) -> Result<LaunchPlan> {
    if settings.server_url.trim().is_empty() {
        return Err(
            "Missing server_url for n8n-mcp. Please set server_url to your instance-level n8n MCP endpoint (e.g. https://<host>/mcp-server/http) in your Zed settings."
                .into(),
        );
    }

    match settings.auth {
        AuthMode::Oauth => Ok(LaunchPlan {
            package: MCP_REMOTE_PACKAGE,
            entry: MCP_REMOTE_ENTRY,
            trailing_args: vec![settings.server_url.clone()],
        }),
        AuthMode::Token => {
            let token = settings.access_token.as_deref().unwrap_or("").trim();
            if token.is_empty() {
                return Err(
                    "auth is set to \"token\" but access_token is empty. Provide a bearer token, or switch auth to \"oauth\"."
                        .into(),
                );
            }
            Ok(LaunchPlan {
                package: SUPERGATEWAY_PACKAGE,
                entry: SUPERGATEWAY_ENTRY,
                trailing_args: vec![
                    "--streamableHttp".to_string(),
                    settings.server_url.clone(),
                    "--header".to_string(),
                    format!("authorization:Bearer {}", token),
                ],
            })
        }
    }
}

struct N8nMcpExtension;

impl N8nMcpExtension {
    fn install_or_update_package(&self, package: &str) -> Result<()> {
        let installed_version = zed::npm_package_installed_version(package)?;
        let latest_version = zed::npm_package_latest_version(package)?;

        if installed_version.is_none() || installed_version != Some(latest_version.clone()) {
            zed::npm_install_package(package, &latest_version)?;
        }

        Ok(())
    }
}

impl zed::Extension for N8nMcpExtension {
    fn new() -> Self {
        Self
    }

    fn context_server_command(
        &mut self,
        _context_server_id: &ContextServerId,
        project: &Project,
    ) -> Result<Command> {
        let settings = ContextServerSettings::for_project("mcp-n8n", project)?;
        let Some(settings) = settings.settings else {
            return Err(
                "Missing settings for n8n-mcp. Please configure server_url in your Zed settings."
                    .into(),
            );
        };
        let settings: N8nMcpSettings =
            serde_json::from_value(settings).map_err(|e| e.to_string())?;

        let plan = resolve_launch(&settings)?;

        self.install_or_update_package(plan.package)?;

        let script_path = env::current_dir()
            .map_err(|e| e.to_string())?
            .join(plan.entry)
            .to_string_lossy()
            .to_string();

        let mut args = vec![script_path];
        args.extend(plan.trailing_args);

        Ok(Command {
            command: zed::node_binary_path()?,
            args,
            env: vec![],
        })
    }

    fn context_server_configuration(
        &mut self,
        _context_server_id: &ContextServerId,
        _project: &Project,
    ) -> Result<Option<ContextServerConfiguration>> {
        Ok(Some(ContextServerConfiguration {
            installation_instructions: include_str!(
                "../configuration/installation_instructions.md"
            )
            .to_string(),
            default_settings: include_str!("../configuration/default_settings.jsonc").to_string(),
            settings_schema: serde_json::to_string(&schemars::schema_for!(N8nMcpSettings))
                .map_err(|e| e.to_string())?,
        }))
    }
}

zed::register_extension!(N8nMcpExtension);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_defaults_to_oauth_when_absent() {
        let s: N8nMcpSettings =
            serde_json::from_str(r#"{ "server_url": "https://n8n.example/mcp-server/http" }"#)
                .unwrap();
        assert_eq!(s.auth, AuthMode::Oauth);
        assert_eq!(s.server_url, "https://n8n.example/mcp-server/http");
        assert_eq!(s.access_token, None);
    }

    #[test]
    fn auth_parses_token_mode() {
        let s: N8nMcpSettings = serde_json::from_str(
            r#"{ "server_url": "https://n8n.example/mcp-server/http", "auth": "token", "access_token": "abc" }"#,
        )
        .unwrap();
        assert_eq!(s.auth, AuthMode::Token);
        assert_eq!(s.access_token.as_deref(), Some("abc"));
    }

    #[test]
    fn unknown_auth_value_is_rejected() {
        let result: Result<N8nMcpSettings> = serde_json::from_str(
            r#"{ "server_url": "https://n8n.example/mcp-server/http", "auth": "magic" }"#,
        )
        .map_err(|e| e.to_string());
        assert!(result.is_err());
    }

    #[test]
    fn oauth_mode_launches_mcp_remote() {
        let s = N8nMcpSettings {
            server_url: "https://n8n.example/mcp-server/http".to_string(),
            auth: AuthMode::Oauth,
            access_token: None,
        };
        let plan = resolve_launch(&s).unwrap();
        assert_eq!(plan.package, "mcp-remote");
        assert_eq!(plan.entry, "node_modules/mcp-remote/dist/proxy.js");
        assert_eq!(
            plan.trailing_args,
            vec!["https://n8n.example/mcp-server/http".to_string()]
        );
    }

    #[test]
    fn token_mode_launches_supergateway_with_header() {
        let s = N8nMcpSettings {
            server_url: "https://n8n.example/mcp-server/http".to_string(),
            auth: AuthMode::Token,
            access_token: Some("secret".to_string()),
        };
        let plan = resolve_launch(&s).unwrap();
        assert_eq!(plan.package, "supergateway");
        assert_eq!(plan.entry, "node_modules/supergateway/dist/index.js");
        assert_eq!(
            plan.trailing_args,
            vec![
                "--streamableHttp".to_string(),
                "https://n8n.example/mcp-server/http".to_string(),
                "--header".to_string(),
                "authorization:Bearer secret".to_string(),
            ]
        );
    }

    #[test]
    fn missing_server_url_is_an_error() {
        let s = N8nMcpSettings {
            server_url: "".to_string(),
            auth: AuthMode::Oauth,
            access_token: None,
        };
        assert!(resolve_launch(&s).is_err());
    }

    #[test]
    fn token_mode_without_token_is_an_error() {
        let s = N8nMcpSettings {
            server_url: "https://n8n.example/mcp-server/http".to_string(),
            auth: AuthMode::Token,
            access_token: Some("   ".to_string()),
        };
        let err = resolve_launch(&s).unwrap_err();
        assert!(err.contains("access_token"));
    }
}
