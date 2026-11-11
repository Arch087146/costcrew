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
  // __dirname is node_modules/@archeagle087/costcrew — two levels up is
  // node_modules, where the platform packages normally live; npm can also
  // nest them under the wrapper's own node_modules
  const nm = path.join(__dirname, "..", "..");
  const nested = path.join(__dirname, "node_modules");
  for (const base of [nm, nested]) {
    const direct = path.join(base, rel);
    if (fs.existsSync(direct)) return direct;
  }
  // last resort: node resolution from both bases
  const [pkg, file] = rel.split("/");
  try {
    const dir = path.dirname(require.resolve(`${pkg}/package.json`, { paths: [nm, __dirname] }));
    return path.join(dir, file);
  } catch {
    return path.join(nm, rel); // let spawn fail with a clear ENOENT
  }
}

const bin = resolveBinary();
const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`costcrew: failed to launch ${bin}: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 0);
