FROM node:lts

ARG TZ
ENV TZ="$TZ"

ARG CLAUDE_CODE_VERSION=latest

# Install basic development tools and iptables/ipset
RUN apt-get update && apt-get install -y --no-install-recommends \
  less \
  git \
  procps \
  sudo \
  fzf \
  zsh \
  unzip \
  gh \
  iptables \
  ipset \
  iproute2 \
  dnsutils \
  aggregate \
  jq \
  && apt-get clean && rm -rf /var/lib/apt/lists/*

# Baseline native-build deps for compiling Rust crates (e.g. anything linking
# OpenSSL). No GUI/tray libs — this is a terminal app, not a Tauri desktop app.
RUN apt-get update && apt-get install -y --no-install-recommends \
  build-essential \
  pkg-config \
  libssl-dev \
  && apt-get clean && rm -rf /var/lib/apt/lists/*

# Install Neovim from upstream's release tarball rather than apt: Debian's
# neovim package lags several major versions behind, and PPAs are Ubuntu-only
# (this image is Debian).
ARG NEOVIM_VERSION=stable
# Neovim's release assets are named by CPU arch (x86_64 / arm64), which differs
# from dpkg's naming (amd64 / arm64) — translate so the arm64 image (e.g. Docker
# Desktop on Apple Silicon) gets a runnable binary instead of an x86_64 one.
RUN case "$(dpkg --print-architecture)" in \
    amd64) NVIM_ARCH=linux-x86_64 ;; \
    arm64) NVIM_ARCH=linux-arm64 ;; \
    *) echo "unsupported arch for neovim: $(dpkg --print-architecture)" >&2; exit 1 ;; \
  esac && \
  wget -O /tmp/nvim.tar.gz "https://github.com/neovim/neovim/releases/download/${NEOVIM_VERSION}/nvim-${NVIM_ARCH}.tar.gz" && \
  tar -C /opt -xzf /tmp/nvim.tar.gz && \
  ln -s "/opt/nvim-${NVIM_ARCH}/bin/nvim" /usr/local/bin/nvim && \
  rm /tmp/nvim.tar.gz

# Ensure default node user has access to /usr/local/share
RUN mkdir -p /usr/local/share/npm-global && \
  chown -R node:node /usr/local/share

ARG USERNAME=node

# Persist bash history.
RUN SNIPPET="export PROMPT_COMMAND='history -a' && export HISTFILE=/commandhistory/.bash_history" \
  && mkdir /commandhistory \
  && touch /commandhistory/.bash_history \
  && chown -R $USERNAME /commandhistory

# Set `DEVCONTAINER` environment variable to help with orientation
ENV DEVCONTAINER=true

# Create workspace and config directories and set permissions.
# /home/node/.config must exist here (not just /home/node/.claude): the nvim
# bind mount and the gh-config named volume in devcontainer.json both target
# subdirs of /home/node/.config, and Docker auto-creates unmounted parent
# dirs (and initializes empty named volumes) as root, blocking tools like
# `gh` that need to mkdir/write under .config as the node user.
RUN mkdir -p /workspace /home/node/.claude /home/node/.config/gh && \
  chown -R node:node /workspace /home/node/.claude /home/node/.config

WORKDIR /workspace

ARG GIT_DELTA_VERSION=0.18.2
RUN ARCH=$(dpkg --print-architecture) && \
  wget "https://github.com/dandavison/delta/releases/download/${GIT_DELTA_VERSION}/git-delta_${GIT_DELTA_VERSION}_${ARCH}.deb" && \
  sudo dpkg -i "git-delta_${GIT_DELTA_VERSION}_${ARCH}.deb" && \
  rm "git-delta_${GIT_DELTA_VERSION}_${ARCH}.deb"

# Set up non-root user
USER node

# Install global packages
ENV NPM_CONFIG_PREFIX=/usr/local/share/npm-global
ENV PATH=$PATH:/usr/local/share/npm-global/bin

# Set the default shell to zsh rather than sh
ENV SHELL=/bin/zsh

# /workspace is a bind mount from the host, so its owning uid rarely matches
# node's — git otherwise refuses to operate on it as "dubious ownership".
RUN git config --global --add safe.directory /workspace

# Set the default editor and visual
ENV EDITOR=nvim
ENV VISUAL=nvim

# Minimal zsh configuration without oh-my-zsh
RUN printf 'setopt SHARE_HISTORY\nsetopt INC_APPEND_HISTORY\nHISTFILE=~/.zsh_history\nHISTSIZE=50000\nSAVEHIST=50000\n' > /home/node/.zshrc && \
  chown node:node /home/node/.zshrc

# Install Rust (stable, unpinned — same rationale as neovim above)
RUN wget -qO- https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
ENV PATH=$PATH:/home/node/.cargo/bin
RUN rustup component add rust-analyzer rustfmt clippy

# Install Claude. openspec is the CLI backing this repo's OpenSpec skills
# (`openspec list --json`, etc.) — the skills ship in the repo but call the
# bare `openspec` binary, so it has to be on PATH.
RUN npm install -g \
  @anthropic-ai/claude-code@${CLAUDE_CODE_VERSION} \
  @agentclientprotocol/claude-agent-acp \
  @fission-ai/openspec@latest


# Copy and set up firewall script
COPY init-firewall.sh /usr/local/bin/
USER root
RUN chmod +x /usr/local/bin/init-firewall.sh

# Named volumes (cargo target/registry, nvim data/state) mount over
# directories owned by root by default; fix ownership on every start.
RUN printf '#!/bin/bash\nchown -R node:node /workspace/target /home/node/.cargo/registry /home/node/.local/share/nvim /home/node/.local/state\n' \
  > /usr/local/bin/fix-workspace-perms.sh && \
  chmod +x /usr/local/bin/fix-workspace-perms.sh

RUN echo "node ALL=(root) NOPASSWD: /usr/local/bin/init-firewall.sh, /usr/local/bin/fix-workspace-perms.sh" > /etc/sudoers.d/node-firewall && \
  chmod 0440 /etc/sudoers.d/node-firewall
USER node
