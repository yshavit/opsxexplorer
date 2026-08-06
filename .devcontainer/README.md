# Claude Code sandbox

## Usage

### One-time setup

1. Create `.devcontainer-extras/nvim-config` at the repo root, as a symlink to
   your real nvim config (or an empty dir if you don't have one):
   - Windows (PowerShell, as admin or with Developer Mode enabled):
     ```powershell
     New-Item -ItemType SymbolicLink -Path .devcontainer-extras\nvim-config -Target $env:LOCALAPPDATA\nvim
     ```
   - mac/linux:
     ```bash
     mkdir -p .devcontainer-extras
     ln -s ~/.config/nvim .devcontainer-extras/nvim-config
     ```
   `.devcontainer-extras/` is gitignored, so this symlink stays host-local.

2. Install devcontainers and just:
   ```shell
   npm i -g @devcontainers/cli   # once
   cargo install just            # once (or: winget install Casey.Just)
   ```

### Each time

```
just claude
```

After editing the Dockerfile or devcontainer.json, rebuild with:

```
just devcontainer-rebuild
```

## Zed ACP integration

Add to your (host) Zed `settings.json`:

```json
"agent_servers": {
  "Claude Agent (Container)": {
    "type": "custom",
    "command": "node",
    "args": ["<path-to-this-checkout>\\.devcontainer\\acp-cwd-proxy.mjs"]
  }
}
```

## Host-specific extras (e.g. statusline)

Drop or symlink host-specific files into `.devcontainer-extras/` at the repo
root (gitignored) — it's part of the workspace, so it's already visible
inside the container at `/workspace/.devcontainer-extras/`. Then, one time,
from inside the container:

```
ln -s /workspace/.devcontainer-extras/statusline.sh ~/.claude/statusline.sh
```

`~/.claude` is a persisted volume, so the symlink survives container
recreation — no need to redo this unless the volume itself is deleted.

## What is this

Based on Anthropic's reference dev container
(https://github.com/anthropics/claude-code/tree/main/.devcontainer), with
neovim, a bind-mounted nvim config, `gh` auth persistence, and a Zed ACP proxy
added. Runs Claude Code and neovim as a non-root user, confined to this repo,
with a default-deny network firewall (`init-firewall.sh`) that only allows
GitHub, npm, crates.io, and Anthropic's API.
