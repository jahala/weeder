#!/usr/bin/env node

"use strict";

const https = require("https");
const http = require("http");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const PLATFORM_MAP = {
  "linux-x64": "x86_64-unknown-linux-musl",
  "linux-arm64": "aarch64-unknown-linux-musl",
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
};

const key = `${process.platform}-${process.arch}`;
const target = PLATFORM_MAP[key];

if (!target) {
  console.error(`weed: no release binary for ${key}`);
  console.error(`Released for: ${Object.keys(PLATFORM_MAP).join(", ")}`);
  console.error("Build it instead: cargo install weed");
  process.exit(1);
}

const version = require("./package.json").version;
const isWindows = process.platform === "win32";
const ext = isWindows ? "zip" : "tar.gz";
const binName = isWindows ? "weed.exe" : "weed";
const url = `https://github.com/jahala/weed/releases/download/v${version}/weed-${target}.${ext}`;

const binDir = path.join(__dirname, "bin");
const binPath = path.join(binDir, binName);

// Already here from an earlier install: nothing to fetch.
if (fs.existsSync(binPath)) {
  process.exit(0);
}

fs.mkdirSync(binDir, { recursive: true });

console.log(`weed: downloading the ${target} binary`);

function follow(url, callback) {
  const mod = url.startsWith("https") ? https : http;
  mod
    .get(url, { headers: { "User-Agent": "weed-npm" } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        follow(res.headers.location, callback);
      } else if (res.statusCode !== 200) {
        console.error(`weed: the download answered HTTP ${res.statusCode}`);
        console.error(`URL: ${url}`);
        console.error("Install it another way: cargo install weed");
        process.exit(1);
      } else {
        callback(res);
      }
    })
    .on("error", (err) => {
      console.error(`weed: the download did not finish: ${err.message}`);
      console.error("Install it another way: cargo install weed");
      process.exit(1);
    });
}

follow(url, (res) => {
  if (isWindows) {
    // tar reads a zip on modern Windows, so there is no archive library here.
    const tmpZip = path.join(binDir, "weed.zip");
    const out = fs.createWriteStream(tmpZip);
    res.pipe(out);
    out.on("finish", () => {
      out.close();
      try {
        execSync(`tar -xf "${tmpZip}" -C "${binDir}"`, { stdio: "ignore" });
        fs.unlinkSync(tmpZip);
        console.log("weed: installed");
      } catch {
        console.error("weed: the archive did not extract. Install it another way: cargo install weed");
        process.exit(1);
      }
    });
  } else {
    const tar = require("child_process").spawn("tar", ["xz", "-C", binDir], {
      stdio: ["pipe", "inherit", "inherit"],
    });
    res.pipe(tar.stdin);
    tar.on("close", (code) => {
      if (code !== 0) {
        console.error("weed: the archive did not extract. Install it another way: cargo install weed");
        process.exit(1);
      }
      fs.chmodSync(binPath, 0o755);
      console.log("weed: installed");
    });
  }
});
