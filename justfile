# just's default recipe shell is `sh`, which isn't on PATH on Windows outside
# WSL. Point it at Git Bash instead so recipes stay POSIX shell on every OS.
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-cu"]

devcontainer-rebuild:
    devcontainer up --workspace-folder . --remove-existing-container --build-no-cache

devcontainer:
    node .devcontainer/ensure-up.js
    devcontainer exec --workspace-folder . zsh

claude:
    node .devcontainer/ensure-up.js
    devcontainer exec --workspace-folder . claude
