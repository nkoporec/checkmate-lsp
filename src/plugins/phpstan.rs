use std::{collections::HashMap, fs::metadata, process::Command, str};

use dashmap::DashMap;
use log::{error, info};
use serde_derive::Deserialize;
use tower_lsp::lsp_types::{DiagnosticSeverity, Url};

use crate::plugins::{Plugin, PluginLineOutput, PluginOutput, PluginSetting, Position};

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

    fn run(&self, plugin_settings: PluginSetting, uri: Url) -> Option<PluginOutput> {
        info!("Running Phpstan");

        // Append file to args.
        let file = uri.to_string().replace("file://", "");
        let mut args = plugin_settings.args.clone();
        args.push(file);

        let output = Command::new(plugin_settings.cmd)
            .args(args)
            .output()
            .expect("failed to execute process");

        let report: PhpstanReport = serde_json::from_slice(&output.stdout).unwrap_or_default();

        let mut plugin_output = PluginOutput::default();
        for file_report in report.files.values() {
            for message in &file_report.messages {
                plugin_output.messages.push(PluginLineOutput {
                    position: Position {
                        line: message.line - 1,
                        column: 1,
                        line_end: message.line - 1,
                        column_end: 1,
                    },
                    text: message.message.clone(),
                    severity: DiagnosticSeverity::ERROR,
                });
            }
        }

        info!("Phpstan ended.");
        Some(plugin_output)
    }
}
