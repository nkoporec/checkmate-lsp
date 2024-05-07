use std::{collections::HashMap, format, fs::metadata, process::Command, str, vec};

use async_trait::async_trait;
use dashmap::DashMap;
use log::{error, info};
use tower_lsp::lsp_types::{Diagnostic, Position, Range};
use tower_lsp::lsp_types::{DiagnosticSeverity, MessageType, Url};
use tower_lsp::{Client};

use crate::plugins::{Plugin, PluginOutput, PluginSetting};
use serde_derive::Deserialize;

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhpcsReport {
    pub files: HashMap<String, FileReport>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileReport {
    pub errors: i64,
    pub warnings: i64,
    pub messages: Vec<FileMessage>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileMessage {
    pub message: String,
    pub source: String,
    pub severity: i64,
    pub fixable: bool,
    #[serde(rename = "type")]
    pub type_field: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Default)]
pub struct PhpcsPlugin;

#[async_trait]
impl Plugin for PhpcsPlugin {
    fn get_plugin_id(&self) -> &str {
        "phpcs"
    }

    fn is_installed(&self, settings: DashMap<String, String>) -> Option<PluginSetting> {
        let project_root = settings
            .get("root_uri")
            .expect("Cant fetch root uri")
            .to_string()
            .replace("file://", "");

        let project_phpcs = format!("{}/vendor/bin/phpcs", project_root);
        let default_args = vec!["--report=json".to_string()];
        let default_filetypes = vec!["php".to_string()];

        if metadata(project_phpcs.clone()).is_ok() {
            info!("Plugin Phpcs found");
            return Some(PluginSetting {
                cmd: project_phpcs,
                args: default_args,
                filetypes: default_filetypes,
            });
        }

        info!("Project PHPCS not found, trying global ...");

        match Command::new("phpcs").spawn() {
            Ok(_) => Some(PluginSetting {
                cmd: "phpcs".to_string(),
                args: default_args,
                filetypes: default_filetypes,
            }),
            Err(e) => {
                if let std::io::ErrorKind::NotFound = e.kind() {
                    error!("Global PHPCS not found");
                    return None;
                }

                error!("Global PHPCS cant be executed.");
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
                format!("Running PHPCS with command {}", plugin_settings.cmd),
            )
            .await;

        let output = Command::new(plugin_settings.cmd)
            .args(args)
            .output()
            .expect("failed to execute process");

        if !output.stderr.is_empty() {
            client
                .log_message(
                    MessageType::ERROR,
                    format!(
                        "PHPCS returned error: {}",
                        str::from_utf8(&output.stderr).unwrap(),
                    ),
                )
                .await;
            return None;
        }

        let report: PhpcsReport = serde_json::from_slice(&output.stdout).unwrap_or_default();

        for file_report in report.files.values() {
            let mut diagnostics = vec![];
            for message in &file_report.messages {
                let mut severity = DiagnosticSeverity::INFORMATION;

                match &message.type_field[..] {
                    "WARNING" => severity = DiagnosticSeverity::WARNING,
                    "ERROR" => severity = DiagnosticSeverity::ERROR,
                    _ => {}
                }

                let item = Diagnostic::new(
                    Range::new(
                        Position {
                            line: message.line - 1,
                            character: message.column,
                        },
                        Position {
                            line: message.line - 1,
                            character: message.column,
                        },
                    ),
                    Some(severity),
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
            .log_message(MessageType::LOG, "PHPCS ended".to_string())
            .await;
        None
    }
}
