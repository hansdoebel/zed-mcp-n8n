use schemars::JsonSchema;
use serde::Deserialize;
use std::env;
use zed_extension_api::{
    self as zed, settings::ContextServerSettings, Command, ContextServerConfiguration,
    ContextServerId, Project, Result,
};

const PACKAGE_NAME: &str = "supergateway";
const SERVER_PATH: &str = "node_modules/supergateway/dist/index.js";

#[derive(Debug, Deserialize, JsonSchema)]
struct N8nMcpSettings {
    server_url: String,
    #[serde(default)]
    access_token: Option<String>,
}

struct N8nMcpExtension;

impl N8nMcpExtension {
    fn install_or_update_package(&self) -> Result<()> {
        let installed_version = zed::npm_package_installed_version(PACKAGE_NAME)?;
        let latest_version = zed::npm_package_latest_version(PACKAGE_NAME)?;

        if installed_version.is_none() || installed_version != Some(latest_version.clone()) {
            zed::npm_install_package(PACKAGE_NAME, &latest_version)?;
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
        self.install_or_update_package()?;

        let settings = ContextServerSettings::for_project("mcp-n8n", project)?;
        let Some(settings) = settings.settings else {
            return Err(
                "Missing settings for n8n-mcp. Please configure server_url in your Zed settings."
                    .into(),
            );
        };
        let settings: N8nMcpSettings =
            serde_json::from_value(settings).map_err(|e| e.to_string())?;

        let server_path = env::current_dir()
            .map_err(|e| e.to_string())?
            .join(SERVER_PATH)
            .to_string_lossy()
            .to_string();

        let mut args = vec![
            server_path,
            "--streamableHttp".to_string(),
            settings.server_url,
        ];

        if let Some(token) = settings.access_token {
            if !token.is_empty() {
                args.push("--header".to_string());
                args.push(format!("authorization:Bearer {}", token));
            }
        }

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
