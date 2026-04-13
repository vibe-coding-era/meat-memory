#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const packageRoot = path.resolve(__dirname, "..");
const binaryName = process.platform === "win32" ? "memory-cli.exe" : "memory-cli";
const binaryPath = process.env.MEAT_MEMORY_BINARY_PATH
  || path.join(packageRoot, "vendor", "bin", binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error("Meat Memory CLI binary was not found.");
  console.error(`Expected: ${binaryPath}`);
  console.error("Set MEAT_MEMORY_BINARY_PATH or configure the npm package releaseBaseUrl before install.");
  process.exit(127);
}

const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: "inherit",
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 0);
