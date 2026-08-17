import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const version = process.argv[2]?.trim().replace(/^v/i, "");

if (!version || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error("用法：npm run version:set -- 0.1.8");
}

async function updateJson(relativePath, update) {
  const filePath = path.join(rootDir, relativePath);
  const data = JSON.parse(await readFile(filePath, "utf8"));
  const before = JSON.stringify(data);
  update(data);
  if (JSON.stringify(data) !== before) {
    await writeFile(filePath, `${JSON.stringify(data, null, 2)}\n`, "utf8");
  }
}

await updateJson("package.json", (data) => {
  data.version = version;
});

await updateJson("package-lock.json", (data) => {
  data.version = version;
  if (!data.packages?.[""]) {
    throw new Error("package-lock.json 缺少根包信息");
  }
  data.packages[""].version = version;
});

await updateJson("src-tauri/tauri.conf.json", (data) => {
  data.version = version;
});

const cargoTomlPath = path.join(rootDir, "src-tauri", "Cargo.toml");
const cargoToml = await readFile(cargoTomlPath, "utf8");
const cargoTomlVersion = /(^\[package\][\s\S]*?^version\s*=\s*")[^"]+("\s*$)/m;
if (!cargoTomlVersion.test(cargoToml)) {
  throw new Error("无法更新 src-tauri/Cargo.toml 的 package.version");
}
const updatedCargoToml = cargoToml.replace(
  cargoTomlVersion,
  (_, prefix, suffix) => `${prefix}${version}${suffix}`,
);
if (updatedCargoToml !== cargoToml) {
  await writeFile(cargoTomlPath, updatedCargoToml, "utf8");
}

const cargoLockPath = path.join(rootDir, "src-tauri", "Cargo.lock");
const cargoLock = await readFile(cargoLockPath, "utf8");
const cargoLockVersion = /(\[\[package\]\]\r?\nname = "z-com"\r?\nversion = ")[^"]+("\r?\n)/;
if (!cargoLockVersion.test(cargoLock)) {
  throw new Error("无法更新 src-tauri/Cargo.lock 中的 z-com 版本");
}
const updatedCargoLock = cargoLock.replace(
  cargoLockVersion,
  (_, prefix, suffix) => `${prefix}${version}${suffix}`,
);
if (updatedCargoLock !== cargoLock) {
  await writeFile(cargoLockPath, updatedCargoLock, "utf8");
}

console.log(`版本已统一更新为 ${version}：`);
console.log("- package.json");
console.log("- package-lock.json");
console.log("- src-tauri/Cargo.toml");
console.log("- src-tauri/Cargo.lock");
console.log("- src-tauri/tauri.conf.json");
