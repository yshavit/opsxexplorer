#!/usr/bin/env node
// Proxies Zed's ACP stdio into `docker exec` for the containerized Claude
// agent. Zed always sends `cwd` as this project's HOST path (it has no idea
// the agent is actually running inside a container via docker exec), so this
// rewrites any `cwd` field to the container's fixed workspace path before
// forwarding each message. ACP is newline-delimited JSON-RPC over stdio, so
// each line is one complete message.
import { spawn, spawnSync } from 'node:child_process';
import readline from 'node:readline';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CONTAINER_CWD = '/workspace';
const REPO_ROOT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));

// Zed spawns this process with a minimal PATH that doesn't include npm's
// global bin dir (unlike `node`, which it finds via a different directory) —
// resolve `devcontainer`'s full path explicitly rather than relying on PATH.
const DEVCONTAINER_BIN = process.platform === 'win32' ? path.join(process.env.APPDATA, 'npm', 'devcontainer.cmd') : 'devcontainer';

// devcontainer up's own output must never touch our process.stdout — that
// channel is the ACP JSON-RPC stream to Zed, and any stray log line would
// corrupt it. Route both its stdout and stderr to our stderr (fd 2) instead,
// since its errors often land on stdout.
const up = spawnSync(DEVCONTAINER_BIN, ['up', '--workspace-folder', REPO_ROOT], {
  stdio: ['ignore', 2, 2],
  shell: process.platform === 'win32', // .cmd shims need a shell to exec
});
if (up.error) {
  console.error('failed to spawn devcontainer:', up.error);
  process.exit(1);
}
if (up.status !== 0) {
  console.error('devcontainer up failed; see above');
  process.exit(up.status ?? 1);
}

const child = spawn('docker', ['exec', '-i', 'opsxexplorer-devcontainer', 'npx', '-y', '@agentclientprotocol/claude-agent-acp'], {
  stdio: ['pipe', 'pipe', 'inherit'],
});

function rewriteCwd(value) {
  if (Array.isArray(value)) return value.map(rewriteCwd);
  if (value && typeof value === 'object') {
    const out = {};
    for (const [key, v] of Object.entries(value)) {
      out[key] = key === 'cwd' && typeof v === 'string' ? CONTAINER_CWD : rewriteCwd(v);
    }
    return out;
  }
  return value;
}

const rl = readline.createInterface({ input: process.stdin, terminal: false });
rl.on('line', (line) => {
  if (!line.trim()) return;
  let msg;
  try {
    msg = JSON.parse(line);
  } catch {
    child.stdin.write(line + '\n');
    return;
  }
  child.stdin.write(JSON.stringify(rewriteCwd(msg)) + '\n');
});
rl.on('close', () => child.stdin.end());

child.stdout.pipe(process.stdout);
child.on('exit', (code, signal) => {
  process.exit(code ?? (signal ? 1 : 0));
});
