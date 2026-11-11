#!/usr/bin/env node
// costcrew npm wrapper: finds the platform binary installed as an
// optionalDependency and execs it with all arguments passed through.
"use strict";
const { spawnSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const PLATFORMS = {
  "win32-x64": "@archeagle087/costcrew-win32-x64/costcrew.exe",
  "linux-x64": "@archeagle087/costcrew-linux-x64/costcrew",
  "darwin-arm64": "@archeagle087/costcrew-darwin-arm64/costcrew",
  "darwin-x64": "@archeagle087/costcrew-darwin-x64/costcrew",
};

const key = `${process.platform}-${process.arch}`;
const rel = PLATFORMS[key];
if (!rel) {
  console.error(`costcrew: no prebuilt binary for ${key}`);
  console.error("build from source: https://github.com/Arch087146/costcrew");
  process.exit(1);
}

function resolveBinary() {
  const direct = path.join(__dirname, "..", rel);
  if (fs.existsSync(direct)) return direct;
  // npm can hoist/reshape optionalDependencies — fall back to resolution
  const [pkg, file] = rel.split("/");
  try {
    const dir = path.dirname(require.resolve(`${pkg}/package.json`, { paths: [__dirname] }));
    return path.join(dir, file);
  } catch {
    return direct; // let spawn fail with a clear ENOENT
  }
}

const bin = resolveBinary();
const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`costcrew: failed to launch ${bin}: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 0);
