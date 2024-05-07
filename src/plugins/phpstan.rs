use std::{collections::HashMap, fs::metadata, process::Command, str};

use async_trait::async_trait;
use dashmap::DashMap;
use log::{error, info};
use serde_derive::Deserialize;
use tower_lsp::lsp_types::{Diagnostic, MessageType, Position, Range};
use tower_lsp::lsp_types::{DiagnosticSeverity, Url};
use tower_lsp::Client;

use crate::plugins::{Plugin, PluginOutput, PluginSetting};

#[derive(Default)]
pub struct PhpstanPlugin;

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhpstanReport {
    pub files: HashMap<String, FileReport>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileReport {
    pub messages: Vec<FileMessage>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileMessage {
    pub message: String,
    pub line: u32,
}

#[async_trait]
impl Plugin for PhpstanPlugin {
    fn get_plugin_id(&self) -> &str {
        "phpstan"
    }

    fn is_installed(&self, settings: DashMap<String, String>) -> Option<PluginSetting> {
        let project_root = settings
            .get("root_uri")
            .expect("Cant fetch root uri")
            .to_string()
            .replace("file://", "");

        let project_phpstan = format!("{}/vendor/bin/phpstan", project_root);
        let default_args = vec!["analyse".to_string(), "--error-format=json".to_string()];
        let default_filetypes = vec!["php".to_string()];

        if metadata(project_phpstan.clone()).is_ok() {
            info!("Plugin Phpstan found");
            return Some(PluginSetting {
                cmd: project_phpstan,
                args: default_args,
                filetypes: default_filetypes,
            });
        }

        info!("Project Phpstan not found, trying global ...");

        match Command::new("phpstan").spawn() {
            Ok(_) => Some(PluginSetting {
                cmd: "phpstan".to_string(),
                args: default_args,
                filetypes: default_filetypes,
            }),
            Err(e) => {
                if let std::io::ErrorKind::NotFound = e.kind() {
                    error!("Global Phpstan not found");
                    return None;
                }

                error!("Global Phpstan cant be executed.");
                None
            }
        }
    }

    async fn run(
        &self,
        plugin_settings: PluginSetting,
        uri: Url,
        client: Client,
    ) -> Option<PluginOutput> {
        // Append file to args.
        let file = uri.to_string().replace("file://", "");
        let mut args = plugin_settings.args.clone();
        args.push(file);

        client
            .log_message(
                MessageType::LOG,
                format!("Running PHPSTAN with command {}", plugin_settings.cmd),
            )
            .await;

        let output = Command::new(plugin_settings.cmd)
            .args(args)
            .output()
            .expect("failed to execute process");

        let report: PhpstanReport = serde_json::from_slice(&output.stdout).unwrap_or_default();

        for file_report in report.files.values() {
            let mut diagnostics = vec![];
            for message in &file_report.messages {
                let item = Diagnostic::new(
                    Range::new(
                        Position {
                            line: message.line - 1,
                            character: 1,
                        },
                        Position {
                            line: message.line - 1,
                            character: 1,
                        },
                    ),
                    Some(DiagnosticSeverity::ERROR),
                    None,
                    None,
                    message.message.clone(),
                    None,
                    None,
                );

                diagnostics.push(item);
            }

            client
                .publish_diagnostics(uri.clone(), diagnostics, Some(1))
                .await;
        }

        client
            .log_message(MessageType::LOG, "PHPSTAN ended".to_string())
            .await;
        None
    }
}
