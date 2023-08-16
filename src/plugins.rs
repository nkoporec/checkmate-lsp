use dashmap::DashMap;
use tower_lsp::lsp_types::{DiagnosticSeverity, Url};

pub mod eslint;
pub mod phpcs;
pub mod phpstan;
pub mod stylelint;

#[derive(Debug, Clone)]
pub struct PluginSetting {
    pub cmd: String,
    pub args: Vec<String>,
    pub filetypes: Vec<String>,
}

impl Default for PluginSetting {
    fn default() -> Self {
        PluginSetting {
            cmd: "".to_string(),
            args: Vec::new(),
            filetypes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PluginOutput {
    pub messages: Vec<PluginLineOutput>,
}

#[derive(Debug, Clone)]
pub struct PluginLineOutput {
    pub position: Position,
    pub text: String,
    pub severity: DiagnosticSeverity,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct Position {
    pub line: u32,
    pub column: u32,
    pub line_end: u32,
    pub column_end: u32,
}

pub trait Plugin {
    // Get plugin id.
    fn get_plugin_id(&self) -> &str;

    // Check is the plugin is installed and can be executed.
    // Return the plugin settings if its installed.
    fn is_installed(&self, settings: DashMap<String, String>) -> Option<PluginSetting>;

    // Run plugin and return an output.
    fn run(&self, plugin_settings: PluginSetting, uri: Url) -> Option<PluginOutput>;
}
