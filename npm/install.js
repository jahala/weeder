#!/usr/bin/env node

"use strict";

const https = require("https");
const http = require("http");
const fs = require("fs");
const path = require("path");

const PLATFORM_MAP = {
  "linux-x64": "x86_64-unknown-linux-musl",
  "linux-arm64": "aarch64-unknown-linux-musl",
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
};

const NAME = "weeder";
const OWNER = "jahala";

// The release artifact, named the way scripts/package-release.sh names it and
// the way garden.json's install.binaries points at it: one gzipped tar per
// platform, carrying the executable, the manifest and the skill. These are
// exported so scripts/check/release.sh can hold the wrapper and the manifest to
// the same names without either of them being read by eye.
function assetName(target) {
  return `${NAME}-${target}.tar.gz`;
}

function assetUrl(target, version) {
  return `https://github.com/${OWNER}/${NAME}/releases/download/v${version}/${assetName(target)}`;
}

function binaryName(platform) {
  return platform === "win32" ? `${NAME}.exe` : NAME;
}

function install() {
  const key = `${process.platform}-${process.arch}`;
  const target = PLATFORM_MAP[key];

  if (!target) {
    console.error(`weeder: no release binary for ${key}`);
    console.error(`Released for: ${Object.keys(PLATFORM_MAP).join(", ")}`);
    console.error("Build it instead: cargo install weeder");
    process.exit(1);
  }

  const version = require("./package.json").version;
  const url = assetUrl(target, version);
  const binDir = path.join(__dirname, "bin");
  const binPath = path.join(binDir, binaryName(process.platform));

  // Already here from an earlier install: nothing to fetch.
  if (fs.existsSync(binPath)) {
    return;
  }

  fs.mkdirSync(binDir, { recursive: true });
  console.log(`weeder: downloading the ${target} artifact`);

  follow(url, (res) => {
    // tar reads a gzipped tar on every platform this publishes for, Windows
    // included, so there is no archive library here.
    const tar = require("child_process").spawn("tar", ["xz", "-C", binDir], {
      stdio: ["pipe", "inherit", "inherit"],
    });
    res.pipe(tar.stdin);
    tar.on("close", (code) => {
      if (code !== 0) {
        console.error("weeder: the archive did not extract. Install it another way: cargo install weeder");
        process.exit(1);
      }
      fs.chmodSync(binPath, 0o755);
      console.log("weeder: installed");
    });
  });
}

function follow(url, callback) {
  const mod = url.startsWith("https") ? https : http;
  mod
    .get(url, { headers: { "User-Agent": "weeder-npm" } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        follow(res.headers.location, callback);
      } else if (res.statusCode !== 200) {
        console.error(`weeder: the download answered HTTP ${res.statusCode}`);
        console.error(`URL: ${url}`);
        console.error("Install it another way: cargo install weeder");
        process.exit(1);
      } else {
        callback(res);
      }
    })
    .on("error", (err) => {
      console.error(`weeder: the download did not finish: ${err.message}`);
      console.error("Install it another way: cargo install weeder");
      process.exit(1);
    });
}

module.exports = { PLATFORM_MAP, assetName, assetUrl, binaryName };

if (require.main === module) {
  install();
}
