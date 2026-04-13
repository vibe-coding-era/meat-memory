const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");

const packageRoot = path.resolve(__dirname, "..");
const packageJson = require(path.join(packageRoot, "package.json"));
const releaseBaseUrl = process.env.MEAT_MEMORY_NPM_RELEASE_BASE_URL
  || process.env.npm_package_config_releaseBaseUrl
  || packageJson.config?.releaseBaseUrl
  || "";

const target = resolveTarget();
const archiveName = `meat-memory-${target}.tar.gz`;
const vendorDir = path.join(packageRoot, "vendor");
const archivePath = path.join(os.tmpdir(), archiveName);
const binaryPath = path.join(vendorDir, "bin", process.platform === "win32" ? "memory-cli.exe" : "memory-cli");

if (!target) {
  console.warn(`Skipping Meat Memory binary install: unsupported platform ${process.platform}/${process.arch}.`);
  process.exit(0);
}

if (!releaseBaseUrl) {
  console.warn("Skipping Meat Memory binary download: releaseBaseUrl is not configured.");
  console.warn("Set MEAT_MEMORY_NPM_RELEASE_BASE_URL or package config releaseBaseUrl before publishing.");
  process.exit(0);
}

const url = `${releaseBaseUrl.replace(/\/$/, "")}/v${packageJson.version}/${archiveName}`;

download(url, archivePath)
  .then(() => {
    fs.rmSync(vendorDir, { recursive: true, force: true });
    fs.mkdirSync(vendorDir, { recursive: true });
    const result = spawnSync("tar", ["-xzf", archivePath, "-C", vendorDir, "--strip-components=1"], {
      stdio: "inherit",
    });
    if (result.status !== 0) {
      process.exit(result.status ?? 1);
    }
    fs.chmodSync(binaryPath, 0o755);
    console.log(`Installed Meat Memory CLI binary to ${binaryPath}`);
  })
  .catch((error) => {
    console.error(error.message);
    process.exit(1);
  });

function resolveTarget() {
  if (process.platform === "darwin" && process.arch === "arm64") return "darwin-arm64";
  if (process.platform === "darwin" && process.arch === "x64") return "darwin-amd64";
  if (process.platform === "linux" && process.arch === "arm64") return "linux-arm64";
  if (process.platform === "linux" && process.arch === "x64") return "linux-amd64";
  return "";
}

function download(url, outputPath) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(outputPath);
    https.get(url, (response) => {
      if (response.statusCode && response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        file.close();
        fs.rmSync(outputPath, { force: true });
        download(response.headers.location, outputPath).then(resolve, reject);
        return;
      }
      if (response.statusCode !== 200) {
        file.close();
        fs.rmSync(outputPath, { force: true });
        reject(new Error(`Failed to download ${url}: HTTP ${response.statusCode}`));
        return;
      }
      response.pipe(file);
      file.on("finish", () => {
        file.close(resolve);
      });
    }).on("error", (error) => {
      file.close();
      fs.rmSync(outputPath, { force: true });
      reject(error);
    });
  });
}
