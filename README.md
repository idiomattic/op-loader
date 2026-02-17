# op-loader
A TUI and CLI tool for managing 1Password secrets as environment variables.

[![CI](https://github.com/idiomattic/op-loader/workflows/CI/badge.svg)](https://github.com/idiomattic/op-loader/actions)

## Overview
`op-loader` provides a terminal UI for browsing your 1Password vaults and configuring which fields to inject as environment variables. Once configured, use the `env` subcommand to load secrets into your shell session.

### Installation
Via Homebrew (macOS/Linux)
```bash
brew tap idiomattic/op-loader
brew install op-loader
```
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
eval "$(op-loader env inject -vv)"
```
Reads your configured mappings and outputs `export` statements. Add this to your shell rc file (`.bashrc`, `.zshrc`, etc.) to load secrets on shell startup.

To reduce repeated authentication prompts, you can cache resolved secrets per account for a short TTL (macOS only):
```bash
eval "$(op-loader env inject --cache-ttl 10m)"
```
You can also control how long op-loader waits for another process to populate the cache (defaults to 5s):
```bash
eval "$(op-loader env inject --cache-ttl 10m --cache-lock-wait 30s)"
```
If you launch multiple shells in parallel (e.g., tmux or zellij layouts), consider increasing the wait to 20-60s to avoid thundering-herd prompts.
Cache files are stored under `$XDG_CACHE_HOME/op_loader` (or `~/.cache/op_loader`). On macOS, cached values are encrypted using a key stored in the system Keychain. DO NOT COMMIT THESE CACHE FILES TO VERSION CONTROL.

Caching strategy (macOS only):
- op-loader resolves each account’s secrets once per run and builds a JSON map of `VAR -> value`.
- The map is cached per account and reused for both export generation and template rendering.
- A global lock prevents duplicate `op inject` calls when multiple shells start in parallel; if the lock can’t be acquired within the wait window, the command returns an error.

This feature may be undesirable for some, but it is not any less-secure than having the secrets available in plaintext in your shell.

### Unset Environment Variables
It may be desirable to clear all managed environment variables from your shell at times (perhaps when running a coding agent).  To do so:
```bash
eval "$(op-loader env unset)"
```
This unsets all *managed* environment variables, but not vars otherwise exported in your shell.

### Template Files
Some config files (like `~/.npmrc`) don't support environment variable interpolation. Use templates to inject secrets directly into these files.

```bash
op-loader template add ~/.npmrc
```
This copies the file to `~/.config/op_loader/templates/` and adds a comment showing available variables. Edit the template to add `{{VAR_NAME}}` placeholders:
```
# op-loader: Available variables: {{GITHUB_TOKEN}}, {{NPM_TOKEN}}
//registry.npmjs.org/:_authToken={{NPM_TOKEN}}
```

Templates are rendered automatically when you run `op-loader env inject`, or manually with:
```bash
op-loader template render
```

Other template commands:
```bash
op-loader template list    # Show managed templates
op-loader template remove ~/.npmrc  # Stop managing a file
```

### Cache Management
Clear cached `op inject` output (all accounts):
```bash
op-loader cache clear
```

Clear a single account cache:
```bash
op-loader cache clear --account <account_id>
```

### Configuration
Show config file location:
```bash
op-loader config path
```
View current settings:
```bash
op-loader config get -k default_account_id
```

## How It Works
1. Use the TUI to browse your 1Password vaults and select fields
2. Map fields to environment variable names (e.g., `op://Personal/GitHub/token` -> `GITHUB_TOKEN`)
3. Mappings are saved to the config file
4. Run `eval "$(op-loader env inject)"` to inject secrets into your shell

### Configuration
Default config location: `~/.config/op_loader/default-config.toml`

#### Available settings
- `default_account_id`: Auto-select this account on startup
- `default_vault_per_account`: Auto-select vault per account on startup
- `inject_vars`: Map of environment variable names to 1Password references
- `templated_files`: Map of file paths to template configurations

## Privacy
All secrets are fetched directly from 1Password via the `op` CLI. No secrets are stored locally - only the references (e.g., `op://vault/item/field`) are saved in your config file.
If you enable caching with `--cache-ttl`, plaintext `op inject` output is stored temporarily in the cache directory.

## License
MIT
