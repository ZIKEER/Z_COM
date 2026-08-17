import { createHash } from "node:crypto";
import { chmod, copyFile, mkdir, readFile, stat } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const windowsOnly = process.argv.includes("--windows");

if (windowsOnly && process.platform !== "win32") {
  throw new Error("pack:win 只能在 Windows 环境执行");
}
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

const versionName = `V${packageJson.version}`;
const appName = `Z_COM_${versionName}`;
const executableExtension = process.platform === "win32" ? ".exe" : "";
const sourceExecutable = path.join(rootDir, "src-tauri", "target", "release", `z-com${executableExtension}`);

const architecture = { x64: "x86_64", arm64: "arm64" }[process.arch] ?? process.arch;
const platformName = process.platform === "darwin" ? "macos" : process.platform;
const distRoot = process.platform === "win32"
  ? path.join(rootDir, "dist")
  : path.join(rootDir, "dist", `${platformName}-${architecture}`);
const outputDir = path.join(distRoot, appName);
const outputExecutable = path.join(outputDir, `Z_COM${executableExtension}`);

console.log(`\nZ_COM Rust 便携版打包`);
console.log(`版本：${versionName}`);
console.log(`输出：${outputDir}\n`);

const tauriCli = path.join(rootDir, "node_modules", "@tauri-apps", "cli", "tauri.js");
const build = spawnSync(process.execPath, [tauriCli, "build", "--no-bundle"], {
  cwd: rootDir,
  stdio: "inherit",
});

if (build.error) {
  throw build.error;
}
if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

await mkdir(outputDir, { recursive: true });
try {
  await copyFile(sourceExecutable, outputExecutable);
} catch (error) {
  if (error?.code === "EBUSY" || error?.code === "EPERM") {
    throw new Error(`无法覆盖 ${outputExecutable}：该程序正在运行，请关闭后重新执行打包命令`, { cause: error });
  }
  throw error;
}
if (process.platform !== "win32") {
  await chmod(outputExecutable, 0o755);
}

const executable = await readFile(outputExecutable);
const executableStat = await stat(outputExecutable);
const sha256 = createHash("sha256").update(executable).digest("hex").toUpperCase();

console.log(`\n打包完成：${outputExecutable}`);
console.log(`文件大小：${(executableStat.size / 1024 / 1024).toFixed(2)} MiB`);
console.log(`SHA-256：${sha256}`);
