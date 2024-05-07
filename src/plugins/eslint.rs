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

pub type EslintReport = Vec<FileReport>;

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileReport {
    pub file_path: String,
    pub messages: Vec<FileMessage>,
    pub suppressed_messages: Vec<Value>,
    pub error_count: i64,
    pub fatal_error_count: i64,
    pub warning_count: i64,
    pub fixable_error_count: i64,
    pub fixable_warning_count: i64,
    pub source: String,
    pub used_deprecated_rules: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileMessage {
    pub rule_id: Value,
    pub fatal: bool,
    pub severity: i64,
    pub message: String,
    pub line: i64,
    pub column: i64,
    pub node_type: Value,
}

#[derive(Default)]
pub struct EslintPlugin;

#[async_trait]
impl Plugin for EslintPlugin {
    fn get_plugin_id(&self) -> &str {
        "eslint"
    }

    fn is_installed(&self, settings: DashMap<String, String>) -> Option<PluginSetting> {
        let project_root = settings
            .get("root_uri")
            .expect("Cant fetch root uri")
            .to_string()
            .replace("file://", "");

        let project_eslint = format!("{}/node_modules/.bin/eslint", project_root);
        let default_args = vec!["-f=json".to_string()];
        let default_filetypes = vec![
            "js".to_string(),
            "tsx".to_string(),
            "vue".to_string(),
            "svelte".to_string(),
        ];

        info!("{project_eslint}");
        if metadata(project_eslint.clone()).is_ok() {
            info!("Plugin ESLint found");
            return Some(PluginSetting {
                cmd: project_eslint,
                args: default_args,
                filetypes: default_filetypes,
            });
        }

        error!("ESLint cant be executed.");
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
                format!("Running ESLInt with command {}", plugin_settings.cmd),
            )
            .await;

        let output = Command::new(plugin_settings.cmd)
            .args(args)
            .output()
            .expect("failed to execute process");

        if !output.stderr.is_empty() {
            error!(
                "ESLint returned error: {}",
                str::from_utf8(&output.stderr).unwrap()
            );

            return None;
        }

        let report: EslintReport = serde_json::from_slice(&output.stdout).unwrap_or_default();

        for file_report in report {
            let mut diagnostics = vec![];
            for message in &file_report.messages {
                let mut severity = DiagnosticSeverity::INFORMATION;

                match &message.severity {
                    1 => severity = DiagnosticSeverity::WARNING,
                    2 => severity = DiagnosticSeverity::ERROR,
                    _ => {}
                }

                let line_as_u32: u32 = message.line.try_into().unwrap();
                let item = Diagnostic::new(
                    Range::new(
                        Position {
                            line: line_as_u32 - 1,
                            character: message.column.try_into().unwrap(),
                        },
                        Position {
                            line: line_as_u32 - 1,
                            character: message.column.try_into().unwrap(),
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
        }

        client
            .log_message(MessageType::LOG, "ESLint ended".to_string())
            .await;
        None
    }
}
