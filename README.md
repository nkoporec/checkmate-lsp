## Checkmate-LSP

A LSP framework to inject a LSP diagnostics. The most common use case is to run linters (ESLint, Stylelint ...) and inject the errors/warnings into your editor.

At the core of Checkmate lies its architectural concept of plugins. Each enabled plugin execute designated commands, parsing their output and subsequently generating LSP diagnostics in the form of errors, warnings, or other relevant feedback. This dynamic system empowers users to individually activate or deactivate specific plugins according to their tailored editor configuration.

Currently we support:

 - [ESLint](https://github.com/eslint/eslint)
 - [Stylelint](https://github.com/stylelint/stylelint)
 - [PHPCS](https://github.com/squizlabs/PHP_CodeSniffer)
 - [PHPstan](https://github.com/phpstan/phpstan)

More plugins will be added over time, contributions are welcomed.

## Motivating

Running linters should be a straightforward task, however, the current landscape presents numerous options for executing specific linters within your preferred code editor. This often entails installing distinct extensions or plugins within your editor. When dealing with multiple programming languages, this approach necessitates the installation of a separate linter for each language, which is suboptimal. The Language Server Protocol (LSP) emerges as a powerful solution, making it a prime candidate for this task.

Checkmate draws significant inspiration from [null-ls](https://github.com/jose-elias-alvarez/null-ls.nvim), regrettably, null-ls is now archived. The distinction between the two lies in the fact that Checkmate functions as an LSP server. This renders it editor-agnostic, allowing it to seamlessly integrate with any editor that supports the LSP protocol, along with all its associated plugins.

## Install

Using cargo

 `cargo install checkmate-lsp`


## Enabling plugins

Users can enable plugins within their editor LSP settings for checkmate. An example for enabling ESLint in Neovim(see below for full example):

```lua
lspconfig.checkmate.setup{
     settings = {
        checkmate = {
          plugins = {
            eslint = {}
          }
        };
     }
}
```

Each plugin ships with a default cmd, arguments and filetypes that the plugin is intended for, this can be overriden by passing the `cmd`, `args` and `filetypes` parameters.

```lua
lspconfig.checkmate.setup{
     settings = {
        checkmate = {
          plugins = {
            eslint = {
              cmd = "/my/custom/path/eslint",
              args = "--custom=true",
              filetypes = "js,svelte"
            }
          }
        };
     }
}
```

The above example will run on every file change of .js or .svelte file the command: `/my/custom/path/eslint --custom=true` and return results via LSP.

## Developing new plugins

We welcome any contributions to support new plugins/linters. You can check the [plugins](https://github.com/nkoporec/checkmate/tree/main/src/plugins) folder for examples. A new plugins also needs to registered at [lsp.rs](https://github.com/nkoporec/checkmate/blob/main/src/lsp.rs#L43).

## Editor Setup

### Neovim

Plugins required:
 - lspconfig (https://github.com/neovim/nvim-lspconfig)

After installing the package, add this to your lua config

```lua
local configs = require 'lspconfig.configs'
local lspconfig = require 'lspconfig'

if not configs.checkmate then
 configs.checkmate = {
   default_config = {
     cmd = {'checkmate-lsp'},
     root_dir = function(pattern)
      local cwd = vim.loop.cwd()
      local root = lspconfig.util.root_pattern('.git')(pattern)

      -- prefer cwd if root is a descendant
      return lspconfig.util.path.is_descendant(cwd, root) and cwd or root
     end,
     settings = {
        checkmate = {
          plugins = {
            phpcs = {
                args = "--standard=PSR12 --extensions=php,module,inc,install,test,profile,theme,css,info,txt,md,yml";
            },
            eslint = {},
            phpstan = {}
          }
        };
     },
   },
 }
end
lspconfig.checkmate.setup{}
```

## Alternatives

- [null-ls](jose-elias-alvarez/null-ls.nvim): Use Neovim as a language server to inject LSP diagnostics, code actions, and more via Lua.

- [efm-langserver](https://github.com/mattn/efm-langserver) and
  [diagnostic-languageserver](https://github.com/iamcco/diagnostic-languageserver):
  general-purpose language servers that can provide formatting and diagnostics
  from CLI output.

- [nvim-lint](https://github.com/mfussenegger/nvim-lint): a Lua plugin that
  focuses on providing diagnostics from CLI output.

- [formatter.nvim](https://github.com/mhartington/formatter.nvim): a Lua plugin
  that (surprise) focuses on formatting.

- [hover.nvim](https://github.com/lewis6991/hover.nvim): Hover plugin framework
  for Neovim.
