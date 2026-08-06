#!/usr/bin/env node

import { execSync } from 'child_process';

try {
  // Check if the container is already running
  const result = execSync('docker ps -f name=opsxexplorer-devcontainer -q 2>nul', {
    stdio: 'pipe',
    encoding: 'utf-8',
  }).trim();

  if (!result) {
    // Container not running; bring it up
    console.log('Starting container...');
    execSync('devcontainer up --workspace-folder .', { stdio: 'inherit' });
  }
} catch (error) {
  // If docker ps fails, try to bring up the container anyway
  console.log('Starting container...');
  execSync('devcontainer up --workspace-folder .', { stdio: 'inherit' });
}
