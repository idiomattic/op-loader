# op-loader
A TUI and CLI tool for managing 1Password secrets as environment variables.

[![CI](https://github.com/idiomattic/op-loader/workflows/CI/badge.svg)](https://github.com/idiomattic/op-loader/actions)

## Overview
`op-loader` provides a terminal UI for browsing your 1Password vaults and configuring which fields to inject as environment variables. Once configured, use the `env` subcommand to load secrets into your shell session.

### Installation

Via Cargo
```bash
cargo install op-loader
```
Or build from source:
```bash
git clone https://github.com/idiomattic/op-loader
cd op-loader
cargo install --path .
```

### Prerequisites
- [1Password CLI](https://developer.1password.com/docs/cli/get-started/) (`op`) must be installed and authenticated

## Usage

### TUI Mode
```bash
op-loader
```
Launch the interactive terminal UI to:
- Browse accounts and vaults
- Search items with fuzzy matching
- Select fields to map to environment variables
- Set default account/vault (persisted across sessions)

#### Navigation
| Key | Action |
|-----|--------|
| `0`, `1`, `2`, `3` | Focus panel (Accounts, Vaults, Items, Details) |
| `j` / `k` or arrows | Navigate lists |
| `Enter` | Select item / confirm |
| `/` | Start fuzzy search |
| `Esc` | Clear search / close modal |
| `f` | Favorite (set as default) account or vault |
| `q` | Quit |

### Inject Environment Variables
```bash
eval "$(op-loader env)"
```
Reads your configured mappings and outputs `export` statements. Add this to your shell rc file (`.bashrc`, `.zshrc`, etc.) to load secrets on shell startup.

### Configuration
Show config file location:
```bash
op-loader config path
```
View current settings:
```bash
op-loader config get -k default_vault_id
op-loader config get -k default_account_id
```

## How It Works
1. Use the TUI to browse your 1Password vaults and select fields
2. Map fields to environment variable names (e.g., `op://Personal/GitHub/token` -> `GITHUB_TOKEN`)
3. Mappings are saved to the config file
4. Run `eval "$(op-loader env)"` to inject secrets into your shell

### Configuration
Default config location: `~/.config/op_loader/default-config.toml`

#### Available settings
- `default_account_id`: Auto-select this account on startup
- `default_vault_id`: Auto-select this vault on startup
- `inject_vars`: Map of environment variable names to 1Password references

## Privacy
All secrets are fetched directly from 1Password via the `op` CLI. No secrets are stored locally - only the references (e.g., `op://vault/item/field`) are saved in your config file.

## License
MIT
