import { createHash } from "node:crypto";
import { chmod, copyFile, mkdir, readFile, readdir, rm, stat, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageJson = JSON.parse(await readFile(path.join(rootDir, "package.json"), "utf8"));
const tauriConfig = JSON.parse(await readFile(path.join(rootDir, "src-tauri", "tauri.conf.json"), "utf8"));
const cargoToml = await readFile(path.join(rootDir, "src-tauri", "Cargo.toml"), "utf8");
const cargoPackage = cargoToml.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);

if (!cargoPackage) {
  throw new Error("无法从 src-tauri/Cargo.toml 读取版本号");
}

const versions = new Set([packageJson.version, tauriConfig.version, cargoPackage[1]]);
if (versions.size !== 1) {
  throw new Error(
    `版本号不一致：package.json=${packageJson.version}，tauri.conf.json=${tauriConfig.version}，Cargo.toml=${cargoPackage[1]}`,
  );
}

const fingerprintInputs = [
  "package.json",
  "package-lock.json",
  "LICENSE",
  "NOTICE",
  "THIRD_PARTY_NOTICES.md",
  "src",
  "static",
  "src-tauri/Cargo.toml",
  "src-tauri/Cargo.lock",
  "src-tauri/build.rs",
  "src-tauri/tauri.conf.json",
  "src-tauri/icons",
  "src-tauri/src",
  "svelte.config.js",
  "tsconfig.json",
  "vite.config.js",
];

async function collectFiles(relativePath) {
  const absolutePath = path.join(rootDir, relativePath);
  const entryStat = await stat(absolutePath);
  if (entryStat.isFile()) return [relativePath];
  const entries = await readdir(absolutePath, { withFileTypes: true });
  const nested = await Promise.all(entries.map((entry) => collectFiles(path.join(relativePath, entry.name))));
  return nested.flat();
}

const fingerprintFiles = (await Promise.all(fingerprintInputs.map(collectFiles)))
  .flat()
  .sort((left, right) => left.localeCompare(right));
const sourceHash = createHash("sha256");
for (const relativePath of fingerprintFiles) {
  sourceHash.update(relativePath.replaceAll("\\", "/"));
  sourceHash.update("\0");
  sourceHash.update(await readFile(path.join(rootDir, relativePath)));
  sourceHash.update("\0");
}
const sourceFingerprint = sourceHash.digest("hex").toUpperCase();

const platform = process.platform === "win32"
  ? { key: "windows-x86_64", source: "z-com.exe", name: "Z_COM-windows-x86_64.exe" }
  : process.platform === "linux"
    ? { key: "linux-x86_64", source: "z-com", name: "Z_COM-linux-x86_64" }
    : null;

if (!platform || process.arch !== "x64") {
  throw new Error(`发布脚本当前只支持 Windows/Linux x86_64，当前环境为 ${process.platform}/${process.arch}`);
}

const tauriCli = path.join(rootDir, "node_modules", "@tauri-apps", "cli", "tauri.js");
const build = spawnSync(process.execPath, [tauriCli, "build", "--no-bundle"], {
  cwd: rootDir,
  stdio: "inherit",
});
if (build.error) throw build.error;
if (build.status !== 0) process.exit(build.status ?? 1);

const releaseDirectory = path.join(rootDir, "dist", "release", `v${packageJson.version}`);
const stateDirectory = path.join(rootDir, "dist", ".release-state", `v${packageJson.version}`);
const sourceExecutable = path.join(rootDir, "src-tauri", "target", "release", platform.source);
const outputExecutable = path.join(releaseDirectory, platform.name);
await mkdir(releaseDirectory, { recursive: true });
await mkdir(stateDirectory, { recursive: true });
for (const runtimeDirectory of ["config", "logs", "locks", ".update"]) {
  await rm(path.join(releaseDirectory, runtimeDirectory), { recursive: true, force: true });
}
for (const entry of await readdir(releaseDirectory, { withFileTypes: true })) {
  if (entry.isDirectory() && /^instance_\d+$/.test(entry.name)) {
    await rm(path.join(releaseDirectory, entry.name), { recursive: true, force: true });
  }
}
try {
  await copyFile(sourceExecutable, outputExecutable);
} catch (error) {
  if (error?.code === "EBUSY" || error?.code === "EPERM") {
    throw new Error(`无法覆盖 ${outputExecutable}：该程序正在运行，请关闭后重新执行打包命令`, { cause: error });
  }
  throw error;
}
if (process.platform === "linux") await chmod(outputExecutable, 0o755);
await writeFile(
  path.join(stateDirectory, `${platform.key}.json`),
  `${JSON.stringify({ version: packageJson.version, platform: platform.key, sourceFingerprint }, null, 2)}\n`,
  "utf8",
);

const expectedAssets = [
  { platform: "windows-x86_64", name: "Z_COM-windows-x86_64.exe" },
  { platform: "linux-x86_64", name: "Z_COM-linux-x86_64" },
];
const manifest = { version: packageJson.version, assets: {} };
const missing = [];

for (const asset of expectedAssets) {
  const filePath = path.join(releaseDirectory, asset.name);
  const statePath = path.join(stateDirectory, `${asset.platform}.json`);
  try {
    const buildState = JSON.parse(await readFile(statePath, "utf8"));
    if (buildState.version !== packageJson.version || buildState.sourceFingerprint !== sourceFingerprint) {
      throw new Error("stale build");
    }
    const content = await readFile(filePath);
    const fileStat = await stat(filePath);
    if (!fileStat.isFile() || fileStat.size === 0) throw new Error("empty");
    manifest.assets[asset.platform] = {
      name: asset.name,
      size: fileStat.size,
      sha256: createHash("sha256").update(content).digest("hex").toUpperCase(),
    };
  } catch {
    missing.push(asset.platform);
    await rm(filePath, { force: true });
    await rm(statePath, { force: true });
  }
}

const manifestPath = path.join(releaseDirectory, "release-manifest.json");
if (missing.length === 0) {
  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  console.log(`\nRelease 已完整生成：${releaseDirectory}`);
  console.log(`清单：${manifestPath}`);
} else {
  await rm(manifestPath, { force: true });
  console.log(`\n已生成 ${platform.key} 发布文件：${outputExecutable}`);
  console.log(`尚缺或需要按当前源码重建：${missing.join("、")}。`);
  console.log("请在对应平台的同一工作区再次执行 npm run pack:release。");
  console.log("缺少全部平台产物时不会生成 release-manifest.json，避免误发布不完整清单。");
}
