use std::{collections::HashMap, vec};

use dashmap::DashMap;
use log::info;
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::plugins::{Plugin, phpcs::PhpcsPlugin, PluginSetting, phpstan::PhpstanPlugin, eslint::EslintPlugin, stylelint::StylelintPlugin};

pub struct Lsp {
    pub client: Client,
    pub client_settings: ClientSettings,
    pub server_settings: ServerSettings,
}

#[derive(Debug, Clone)]
pub struct ClientSettings {
    pub plugins: Vec<String>,
    pub settings: DashMap<String, String>,
}

impl ClientSettings {
    pub fn new() -> Self {
        ClientSettings {
            plugins: vec![],
            settings: DashMap::new(),
        }
    }
}

pub struct ServerSettings {
    pub available_plugins: HashMap<String, Box<dyn Plugin + Send + Sync>>,
    pub installed_plugins: DashMap<String, PluginSetting>,
    pub debug: bool,
}

impl ServerSettings {
    pub fn new(debug: bool) -> Self {
        let mut available_plugins: HashMap<String, Box<dyn Plugin + Send + Sync>> = HashMap::new();

        // All supported plugins.
        available_plugins.insert(String::from("phpcs"), Box::<PhpcsPlugin>::default());
        available_plugins.insert(String::from("phpstan"), Box::<PhpstanPlugin>::default());
        available_plugins.insert(String::from("eslint"), Box::<EslintPlugin>::default());
        available_plugins.insert(String::from("stylelint"), Box::<StylelintPlugin>::default());

        ServerSettings {
            available_plugins,
            installed_plugins: DashMap::new(),
            debug
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Lsp {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.client_settings.settings.insert("root_uri".to_string(), params.root_uri.unwrap().to_string());
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        // parse editor settings.
        let editor_settings_items = ConfigurationItem{
            scope_uri: None,
            section: Some("checkmate.plugins".to_string()),
        };

        let editor_settings = self.client.configuration(vec![editor_settings_items])
            .await
            .expect("Cant fetch code editor config.");

        let editor_plugins = parse_client_editor_settings(editor_settings);

        for (plugin_id, settings) in editor_plugins {
            let plugin_discovered = self.server_settings.available_plugins.get(&plugin_id);

            if plugin_discovered.is_none() {
                self.client
                    .log_message(MessageType::ERROR, format!("{} plugin does not exist.", plugin_id))
                    .await;
                info!("{} plugin does not exist.", plugin_id);

                continue;
            }

            let plugin = plugin_discovered.unwrap();

            if let Some(default_plugin_setting) = plugin.is_installed(self.client_settings.settings.clone()) {

                if self.server_settings.debug {
                    self.client
                        .log_message(MessageType::ERROR, format!("Plugin {} is installed, executable path is {}", plugin_id, default_plugin_setting.cmd))
                        .await;
                    info!("Plugin {} is installed, executable path is {}", plugin_id, default_plugin_setting.cmd);
                }

                let mut plugin_settings = PluginSetting::default();

                // CMD
                if !settings.cmd.is_empty() {
                    plugin_settings.cmd = settings.cmd.clone();
                }
                else {
                    plugin_settings.cmd = default_plugin_setting.cmd.clone();
                }
    
                // ARGS.
                let mut plugin_args = default_plugin_setting.args.clone();
                for arg in settings.args {
                    plugin_args.push(arg);
                }
                plugin_settings.args = plugin_args;

                // Filetypes.
                if !settings.filetypes.is_empty() {
                    let mut plugin_filetypes = default_plugin_setting.filetypes.clone();
                    for i in settings.filetypes {
                        plugin_filetypes.push(i);
                    }
                    plugin_settings.filetypes = plugin_filetypes;
                }
                else {
                    plugin_settings.filetypes = default_plugin_setting.filetypes.clone();
                }

                self.server_settings.installed_plugins.insert(plugin_id, plugin_settings);
                continue;
            }


            if self.server_settings.debug {
                self.client
                    .log_message(MessageType::ERROR, format!("{} plugin is not installed or can't be executed.", plugin_id))
                    .await;
                info!("{} plugin is not installed or can't be executed.", plugin_id);
            }
        }

        self.client
            .log_message(MessageType::INFO, "checkmate initialized!")
            .await;
    }

    async fn hover(&self, _params: HoverParams) -> Result<Option<Hover>> {
        Ok(None)
    }

    async fn code_action(&self, _params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        Ok(None)
    }

    async fn goto_definition(
        &self,
        _params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        Ok(None)
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change(&self, _params: DidChangeTextDocumentParams) {
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let file_uri = params.text_document.uri.clone();

        if self.server_settings.debug {
            info!("Text saved, running linters...");

            self.client
                .log_message(MessageType::INFO, "Text saved, running linters...")
                .await;
        }

        let mut diagnostics: Vec<Diagnostic> = vec![];
        for (id, settings) in self.server_settings.installed_plugins.clone() {
            let plugin = self.server_settings.available_plugins.get(&id).unwrap();

            if self.server_settings.debug {
                info!("Running plugin: {}", plugin.get_plugin_id());

                self.client
                    .log_message(MessageType::INFO, format!("Running plugin: {}", plugin.get_plugin_id()))
                    .await;
            }

            // Validate filetypes.
            if !settings.filetypes.contains(&file_uri.to_file_path().unwrap().extension().unwrap().to_str().unwrap().to_string()) {
                if self.server_settings.debug {
                    info!("Invalid filetype, allowed filetypes for this plugin {} are: {:?}", id, settings.filetypes);

                    self.client
                        .log_message(MessageType::INFO, format!("Invalid filetype, allowed filetypes for this plugin {} are: {:?}", id, settings.filetypes))
                        .await;
                }

                continue;
            }
            
            let output = plugin.run(settings, params.text_document.uri.clone());
            
            if output.is_none() {
                continue;
            }

            let messages = output.clone().unwrap().messages;

            for message in messages {
                let item = Diagnostic::new(
                    Range::new(
                        Position {
                            line: message.position.line,
                            character: message.position.column,
                        },
                        Position {
                            line: message.position.line_end,
                            character: message.position.column_end,
                        },
                    ),
                    Some(message.severity),
                    None,
                    None,
                    message.text,
                    None,
                    None
                );

                diagnostics.push(item);
            }
        }

        self.client.publish_diagnostics(params.text_document.uri.clone(), diagnostics, Some(1))
        .await;
    }
}

fn parse_client_editor_settings(config: Vec<Value>) -> HashMap<String, PluginSetting> {
    let mut editor_plugins: HashMap<String, PluginSetting> = HashMap::new();
    for mut item in config {
        if item.as_object_mut().is_none() {
            continue;
        }

        let settings_object = item.as_object_mut().expect("Settings are not an object.");

        for id in settings_object.keys() {
            let user_defined_settings_object = settings_object.get(id).unwrap().as_object();

            if user_defined_settings_object.is_none() {
                editor_plugins.insert(
                    id.to_owned(),
                    PluginSetting::default(),
                );
                continue;
            }

            let user_defined_settings = user_defined_settings_object.unwrap();
            let cmd = user_defined_settings.get("cmd").unwrap_or(&Value::String("".to_string())).as_str().unwrap_or("").to_string();
            let args = user_defined_settings.get("args").unwrap_or(&Value::String("".to_string())).as_str().unwrap_or("").to_string();
            let filetypes = user_defined_settings.get("filetypes").unwrap_or(&Value::String("".to_string())).as_str().unwrap_or("").to_string();

            let mut args_vec = vec![];
            args
                .split(' ')
                .for_each(|i| {
                    args_vec.push(i.to_string());
                });

            let mut filetypes_vec = vec![];
            filetypes
                .split(',')
                .for_each(|i| {
                    filetypes_vec.push(i.to_string());
                });

            editor_plugins.insert(
                id.to_owned(),
                PluginSetting { 
                    cmd,
                    args: args_vec,
                    filetypes: filetypes_vec,
                }
            );
        }
    }

    editor_plugins
}
