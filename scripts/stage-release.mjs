import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import path from "node:path";
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

const version = packageJson.version;
const portableDirectory = `Z_COM_V${version}`;
const releaseDirectory = path.join(rootDir, "dist", "release", `v${version}`);
const legalFiles = ["LICENSE", "NOTICE", "THIRD_PARTY_NOTICES.md"];
const assets = [
  {
    platform: "windows-x86_64",
    source: path.join(rootDir, "dist", portableDirectory, "Z_COM.exe"),
    name: "Z_COM-windows-x86_64.exe",
  },
  {
    platform: "linux-x86_64",
    source: path.join(rootDir, "dist", "linux-x86_64", portableDirectory, "Z_COM"),
    name: "Z_COM-linux-x86_64",
  },
];

for (const asset of assets) {
  let sourceStat;
  try {
    sourceStat = await stat(asset.source);
  } catch {
    throw new Error(`缺少 ${asset.platform} 绿色版主程序：${asset.source}`);
  }
  if (!sourceStat.isFile() || sourceStat.size === 0) {
    throw new Error(`${asset.platform} 绿色版主程序无效：${asset.source}`);
  }
}

await rm(releaseDirectory, { recursive: true, force: true });
await mkdir(releaseDirectory, { recursive: true });
for (const legalFile of legalFiles) {
  await copyFile(path.join(rootDir, legalFile), path.join(releaseDirectory, legalFile));
}

const manifest = { version, assets: {} };
for (const asset of assets) {
  const destination = path.join(releaseDirectory, asset.name);
  await copyFile(asset.source, destination);
  const content = await readFile(destination);
  const destinationStat = await stat(destination);
  manifest.assets[asset.platform] = {
    name: asset.name,
    size: destinationStat.size,
    sha256: createHash("sha256").update(content).digest("hex").toUpperCase(),
  };
}

const manifestPath = path.join(releaseDirectory, "release-manifest.json");
await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");

console.log(`Release 资产已整理到：${releaseDirectory}`);
for (const [platform, asset] of Object.entries(manifest.assets)) {
  console.log(`${platform}: ${asset.name} (${asset.size} bytes)`);
  console.log(`SHA-256: ${asset.sha256}`);
}
console.log(`清单：${manifestPath}`);
