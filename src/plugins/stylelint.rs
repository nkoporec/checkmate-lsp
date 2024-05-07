use std::{format, fs::metadata, process::Command, str, vec};

use async_trait::async_trait;
use dashmap::DashMap;
use log::{error, info};
use serde_json::Value;
use tower_lsp::lsp_types::{Diagnostic, Position, Range};
use tower_lsp::lsp_types::{DiagnosticSeverity, MessageType, Url};
use tower_lsp::Client;

use crate::plugins::{Plugin, PluginOutput, PluginSetting};
use serde_derive::Deserialize;

pub type StylelintReport = Vec<FileReport>;

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileReport {
    pub source: String,
    pub deprecations: Vec<Value>,
    pub invalid_option_warnings: Vec<Value>,
    pub parse_errors: Vec<Value>,
    pub errored: bool,
    pub warnings: Vec<FileMessage>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileMessage {
    pub line: i64,
    pub column: i64,
    pub end_line: i64,
    pub end_column: i64,
    pub rule: String,
    pub severity: String,
    pub text: String,
}

#[derive(Default, Deserialize)]
pub struct StylelintPlugin;

#[async_trait]
impl Plugin for StylelintPlugin {
    fn get_plugin_id(&self) -> &str {
        "stylelint"
    }

    fn is_installed(&self, settings: DashMap<String, String>) -> Option<PluginSetting> {
        let project_root = settings
            .get("root_uri")
            .expect("Cant fetch root uri")
            .to_string()
            .replace("file://", "");

        let project_stylelint = format!("{}/node_modules/.bin/stylelint", project_root);
        let default_args = vec!["-f=json".to_string()];
        let default_filetypes = vec!["css".to_string(), "less".to_string(), "sass".to_string()];

        if metadata(project_stylelint.clone()).is_ok() {
            info!("Plugin Stylelint found");
            return Some(PluginSetting {
                cmd: project_stylelint,
                args: default_args,
                filetypes: default_filetypes,
            });
        }

        error!("Stylelint cant be executed.");
        None
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
            error!(
                "Stylelint returned error: {}",
                str::from_utf8(&output.stderr).unwrap()
            );

            return None;
        }

        let report: StylelintReport = serde_json::from_slice(&output.stdout).unwrap_or_default();

        for file_report in report {
            let mut diagnostics = vec![];
            for message in &file_report.warnings {
                let mut severity = DiagnosticSeverity::INFORMATION;

                match &message.severity[..] {
                    "warning" => severity = DiagnosticSeverity::WARNING,
                    "error" => severity = DiagnosticSeverity::ERROR,
                    _ => {}
                }

                let line_as_u32: u32 = message.line.try_into().unwrap();
                let end_line_as_u32: u32 = message.end_line.try_into().unwrap();

                let item = Diagnostic::new(
                    Range::new(
                        Position {
                            line: line_as_u32 - 1,
                            character: message.column.try_into().unwrap(),
                        },
                        Position {
                            line: end_line_as_u32 - 1,
                            character: message.column.try_into().unwrap(),
                        },
                    ),
                    Some(severity),
                    None,
                    None,
                    message.text.clone(),
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
            .log_message(MessageType::LOG, "Stylelint ended".to_string())
            .await;
        None
    }
}
