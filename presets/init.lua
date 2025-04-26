-- Basic settings
vim.opt.compatible = false
vim.opt.encoding = 'utf-8'
vim.opt.background = 'dark'
vim.opt.number = true
vim.opt.tabstop = 4
vim.opt.shiftwidth = 4
vim.opt.expandtab = true
vim.opt.autoindent = true
vim.opt.copyindent = true
vim.opt.hlsearch = true
vim.opt.scrolloff = 5
vim.opt.splitbelow = true
vim.opt.splitright = true
vim.opt.wildmenu = true
vim.opt.showmatch = true
vim.opt.termguicolors = true
vim.opt.spell = true
vim.opt.swapfile = false
vim.opt.backup = false
vim.opt.undolevels = 5000
vim.opt.history = 5000
vim.opt.autoread = true
vim.opt.backspace = 'indent,eol,start'
vim.opt.signcolumn = 'yes'
vim.opt.cursorline = true
vim.opt.wrap = false
vim.opt.ignorecase = true
vim.opt.smartcase = true

-- Bootstrap packer
local install_path = vim.fn.stdpath('data')..'/site/pack/packer/start/packer.nvim'
if vim.fn.empty(vim.fn.glob(install_path)) > 0 then
  vim.fn.system({'git', 'clone', '--depth', '1', 'https://github.com/wbthomason/packer.nvim', install_path})
  vim.cmd('packadd packer.nvim')
end

-- Plugin setup
require('packer').startup(function(use)
    use 'wbthomason/packer.nvim'

    -- Color scheme
    use 'sainnhe/gruvbox-material'
    use 'folke/tokyonight.nvim'

    -- Mason for automatic LSP installation
    use 'williamboman/mason.nvim'
    use 'williamboman/mason-lspconfig.nvim'

    -- LSP and completion
    use 'neovim/nvim-lspconfig'
    use 'hrsh7th/nvim-cmp'
    use 'hrsh7th/cmp-nvim-lsp'
    use 'hrsh7th/cmp-buffer'
    use 'hrsh7th/cmp-path'
    use 'L3MON4D3/LuaSnip'
    use 'saadparwaiz1/cmp_luasnip'
    use 'rafamadriz/friendly-snippets'
    use 'onsails/lspkind.nvim'

    -- Programming language support
    -- Python
    use 'Vimjas/vim-python-pep8-indent'
    
    -- C++ enhancements
    use 'p00f/clangd_extensions.nvim'
    use 'octol/vim-cpp-enhanced-highlight'
    
    -- Rust
    use 'simrat39/rust-tools.nvim'
    
    -- Go
    use 'fatih/vim-go'
    
    -- Markdown
    use {
        'iamcco/markdown-preview.nvim',
        run = function() vim.fn['mkdp#util#install']() end,
    }

    -- Formatter
    use 'stevearc/conform.nvim'

    -- Treesitter with improved installation
    use {
        'nvim-treesitter/nvim-treesitter',
        run = function()
            -- Try to handle both macOS and Linux environments
            local ts_update = require('nvim-treesitter.install').update({ with_sync = true })
            ts_update()
        end,
        config = function()
            require('nvim-treesitter.configs').setup {
                ensure_installed = { "lua", "rust", "cpp", "c", "python", "go", "javascript", "typescript", "html", "css", "json", "markdown" },
                highlight = {
                    enable = true,
                    additional_vim_regex_highlighting = false,
                },
                indent = {
                    enable = true
                },
                -- Compiler settings that work on both macOS and Linux
                compiler_options = {
                    on_the_fly = true,
                },
                auto_install = true
            }
        end
    }
    
    -- UI enhancements
    use {
        'nvim-tree/nvim-tree.lua',
        requires = 'nvim-tree/nvim-web-devicons'
    }
    use 'nvim-lualine/lualine.nvim'
    use 'lukas-reineke/indent-blankline.nvim'
    use 'lewis6991/gitsigns.nvim'
    
    -- Fuzzy finder
    use {
      'nvim-telescope/telescope.nvim',
      requires = { 'nvim-lua/plenary.nvim' }
    }
    
    -- Copilot
    use 'github/copilot.vim'
    
    -- UI helpers
    use 'j-hui/fidget.nvim'
    use {
      'folke/trouble.nvim',
      requires = 'nvim-tree/nvim-web-devicons'
    }

    use {
      "mfussenegger/nvim-lint",
      config = function()
        require("lint").linters_by_ft = {
          lua = { "selene" },
        }

        vim.api.nvim_create_autocmd({ "BufWritePost" }, {
          callback = function()
            require("lint").try_lint()
          end,
        })
      end
    }

    -- Testing
    use 'vim-test/vim-test'
    use {
      'nvim-neotest/neotest',
      requires = {
        'nvim-lua/plenary.nvim',
        'antoinemadec/FixCursorHold.nvim',
        'nvim-neotest/neotest-vim-test'
      }
    }
end)

-- Set up Mason first to ensure language servers are available
require("mason").setup({
    ui = {
        icons = {
            package_installed = "✓",
            package_pending = "➜",
            package_uninstalled = "✗"
        }
    }
})

require("mason-lspconfig").setup({
    ensure_installed = { 
        "lua_ls",     -- Lua
        "rust_analyzer", -- Rust
        "gopls",      -- Go
        "clangd",     -- C/C++
        "pyright",    -- Python
    },
    automatic_installation = true,
})

-- Colors
vim.cmd([[if has("termguicolors") | set termguicolors | endif]])
vim.cmd([[colorscheme gruvbox-material]])
vim.g.gruvbox_material_background = 'medium'
vim.g.gruvbox_material_enable_italic = 1
vim.g.gruvbox_material_better_performance = 1

-- NvimTree
require('nvim-tree').setup({
    filters = {
        dotfiles = false,
    },
    view = {
        width = 30,
    },
})

-- Status line
require('lualine').setup({
    options = {
        theme = 'gruvbox-material',
        component_separators = { left = '', right = ''},
        section_separators = { left = '', right = ''},
    }
})

-- Git signs
require('gitsigns').setup()

-- Indentation guides (updated for indent-blankline v3)
require('ibl').setup({
    indent = {
        char = '│',
    },
    scope = {
        enabled = true,
        show_start = true,
    }
})

-- LSP configuration
local lspconfig = require('lspconfig')

-- Shared capabilities for all LSP servers
local capabilities = require('cmp_nvim_lsp').default_capabilities()

-- LSP server setups are now handled by mason-lspconfig
-- Mason handles installation, we just need to set up configuration
require("mason-lspconfig").setup_handlers({
    -- Default handler for all servers
    function(server_name)
        lspconfig[server_name].setup({
            capabilities = capabilities,
        })
    end,
    
    -- Custom handler for specific servers
    ["rust_analyzer"] = function()
        require("rust-tools").setup({
            server = {
                capabilities = capabilities,
                settings = {
                    ["rust-analyzer"] = {
                        checkOnSave = {
                            command = "clippy",
                        },
                    },
                },
            },
        })
    end,
    
    ["clangd"] = function()
        require("clangd_extensions").setup({
            server = {
                capabilities = capabilities,
                cmd = {"clangd", "--background-index"}
            },
        })
    end,    
})

-- LSP keybindings with fix for formatting function
vim.api.nvim_create_autocmd('LspAttach', {
  group = vim.api.nvim_create_augroup('UserLspConfig', {}),
  callback = function(ev)
    local opts = { buffer = ev.buf, noremap = true, silent = true }
    vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
    vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
    vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
    vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, opts)
    vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
    -- Use format() instead of formatting which is deprecated
    vim.keymap.set('n', '<leader>f', function() vim.lsp.buf.format({ async = true }) end, opts)
  end,
})

-- Completion setup
local cmp = require('cmp')
local luasnip = require('luasnip')
local lspkind = require('lspkind')

-- Load snippets
require("luasnip.loaders.from_vscode").lazy_load()

cmp.setup({
  snippet = {
    expand = function(args)
      luasnip.lsp_expand(args.body)
    end,
  },
  mapping = cmp.mapping.preset.insert({
    ['<C-Space>'] = cmp.mapping.complete(),
    ['<CR>'] = cmp.mapping.confirm({ select = true }),
    ['<Tab>'] = cmp.mapping(function(fallback)
      if cmp.visible() then
        cmp.select_next_item()
      elseif luasnip.expand_or_jumpable() then
        luasnip.expand_or_jump()
      else
        fallback()
      end
    end, { 'i', 's' }),
    ['<S-Tab>'] = cmp.mapping(function(fallback)
      if cmp.visible() then
        cmp.select_prev_item()
      elseif luasnip.jumpable(-1) then
        luasnip.jump(-1)
      else
        fallback()
      end
    end, { 'i', 's' }),
  }),
  sources = cmp.config.sources({
    { name = 'nvim_lsp' },
    { name = 'luasnip' },
    { name = 'buffer' },
    { name = 'path' },
  }),
  formatting = {
    format = lspkind.cmp_format({
      mode = 'symbol_text',
      maxwidth = 50,
    })
  }
})

-- Formatter with custom markdown config
require("conform").setup({
    format_on_save = false,
    formatters_by_ft = {
        cpp = { "clang-format" },
        rust = { "rustfmt" },
        go = { "gofmt" },
        python = { "black" },
        html = { "prettier" },
        css = { "prettier" },
        javascript = { "prettier" },
        typescript = { "prettier" },
        json = { "prettier" },
        markdown = { "prettier_markdown" },
    },
    formatters = {
        prettier_markdown = {
            command = "prettier",
            args = { "--prose-wrap", "always", "--print-width", "80" },
        },
    },
})

-- Create Format command
vim.api.nvim_create_user_command("Format", function()
    require("conform").format({ async = true })
end, {})

-- Test Runner
vim.g['test#cpp#runner'] = 'gtest'
vim.api.nvim_set_keymap('n', '<leader>t', ':TestNearest<CR>', {noremap = true})
vim.api.nvim_set_keymap('n', '<leader>T', ':TestFile<CR>', {noremap = true})

-- Telescope
local telescope = require('telescope')
telescope.setup()

-- Key Mappings
vim.g.mapleader = " "
vim.api.nvim_set_keymap('n', '<C-n>', ':NvimTreeToggle<CR>', {noremap = true, silent = true})
vim.api.nvim_set_keymap('n', '<leader>ff', ':Telescope find_files<CR>', {noremap = true})
vim.api.nvim_set_keymap('n', '<leader>fg', ':Telescope live_grep<CR>', {noremap = true})
vim.api.nvim_set_keymap('n', '<leader>fb', ':Telescope buffers<CR>', {noremap = true})
vim.api.nvim_set_keymap('n', '<leader>fh', ':Telescope help_tags<CR>', {noremap = true})
vim.api.nvim_set_keymap('n', '<leader>f', ':Format<CR>', {noremap = true})

-- Fidget setup for LSP progress
require("fidget").setup({})

-- Trouble setup for diagnostics
require("trouble").setup({
  icons = false,
  fold_open = "v",
  fold_closed = ">",
  signs = {
    error = "E",
    warning = "W",
    hint = "H",
    information = "I"
  },
})

-- Special file type mappings
vim.api.nvim_create_autocmd({ "BufRead", "BufNewFile" }, {
  pattern = { "*.ixx", "*.cppm", "*.mxx" },
  callback = function()
    vim.bo.filetype = "cpp"
  end
})

-- Detect OS and adapt configuration if needed
local is_mac = vim.fn.has('macunix') == 1
local is_linux = vim.fn.has('unix') == 1 and vim.fn.has('macunix') == 0

if is_mac then
  -- macOS specific settings
  vim.g.python3_host_prog = '/usr/local/bin/python3'
elseif is_linux then
  -- Linux specific settings
  vim.g.python3_host_prog = '/usr/bin/python3'
end
