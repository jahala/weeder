#!/usr/bin/env node

"use strict";

const https = require("https");
const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const PLATFORM_MAP = {
  "linux-x64": "x86_64-unknown-linux-musl",
  "linux-arm64": "aarch64-unknown-linux-musl",
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
};

const NAME = "weeder";
// Where every release's tarballs are published, and where a person goes for
// weeder when this wrapper cannot fetch it. No registry carries weeder yet.
const RELEASES = "https://github.com/jahala/weeder/releases";
// GitHub answers a release download with one redirect to its storage. Five
// leaves room for a moved repository and stops a loop.
const MAX_REDIRECTS = 5;

// The release artifact, named the way scripts/package-release.sh names it and
// the way garden.json's install.binaries points at it: one gzipped tar per
// platform, carrying the executable, the manifest and the skill, with its
// SHA-256 published beside it. These are exported so scripts/check/release.sh
// can hold the wrapper and the manifest to the same names, and run the pieces
// below that decide what reaches the disk.
function assetName(target) {
  return `${NAME}-${target}.tar.gz`;
}

function assetUrl(target, version) {
  return `${RELEASES}/download/v${version}/${assetName(target)}`;
}

function binaryName(platform) {
  return platform === "win32" ? `${NAME}.exe` : NAME;
}

// The digest file is what `sha256sum` writes: the hex digest, then the asset's
// name. A file that names another asset is a digest for something else.
function statedDigest(text, asset) {
  const [hex, named] = String(text).trim().split(/\s+/);
  if (!/^[0-9a-f]{64}$/.test(hex || "")) {
    return { ok: false, reason: `${asset}.sha256 carries no SHA-256` };
  }
  if ((named || "").replace(/^\*/, "") !== asset) {
    return { ok: false, reason: `${asset}.sha256 is the digest of ${named || "nothing named"}` };
  }
  return { ok: true, hex };
}

function checkDigest(archive, digestText, asset) {
  const stated = statedDigest(digestText, asset);
  if (!stated.ok) {
    return stated;
  }
  const actual = crypto.createHash("sha256").update(archive).digest("hex");
  if (actual !== stated.hex) {
    return {
      ok: false,
      reason: `${asset} hashes to ${actual}, and the release states ${stated.hex}, so it is not the archive the release published`,
    };
  }
  return { ok: true };
}

// Where a redirect leads, if it is followed at all: https only, and no more
// than MAX_REDIRECTS of them. `hops` is how many were followed before this one.
function redirectTo(from, location, hops) {
  if (!location) {
    return { ok: false, reason: `${from} redirected without saying where` };
  }
  if (hops >= MAX_REDIRECTS) {
    return { ok: false, reason: `the download redirected more than ${MAX_REDIRECTS} times` };
  }
  let next;
  try {
    next = new URL(location, from);
  } catch (error) {
    return { ok: false, reason: `${from} redirected to ${location}, which is not a url` };
  }
  if (next.protocol !== "https:") {
    return { ok: false, reason: `${from} redirected to ${next.href}, which is not https` };
  }
  return { ok: true, url: next.href };
}

// The archive is checked against its digest before anything is written, and
// then tar is asked for the one executable by name, so no other member of the
// archive reaches the disk. tar reads a gzipped tar from stdin on every
// platform this publishes for, Windows included, so there is no archive
// library here.
function installFrom(archive, digestText, asset, binDir, executable) {
  const checked = checkDigest(archive, digestText, asset);
  if (!checked.ok) {
    return checked;
  }
  fs.mkdirSync(binDir, { recursive: true });
  const tar = spawnSync("tar", ["xzf", "-", "-C", binDir, executable], {
    input: archive,
    stdio: ["pipe", "inherit", "pipe"],
  });
  if (tar.error) {
    return { ok: false, reason: `tar did not run: ${tar.error.message}` };
  }
  const binPath = path.join(binDir, executable);
  if (tar.status !== 0 || !fs.existsSync(binPath)) {
    return {
      ok: false,
      reason: `tar could not take ${executable} out of ${asset}: ${String(tar.stderr).trim()}`,
    };
  }
  fs.chmodSync(binPath, 0o755);
  return { ok: true, path: binPath };
}

// One https GET, its redirects followed under redirectTo's policy, the body
// held in memory. An archive is a few megabytes, and holding it is what lets
// the digest be checked before tar sees a byte.
function download(url, hops = 0) {
  return new Promise((resolve) => {
    const request = https.get(url, { headers: { "User-Agent": "weeder-npm" } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400) {
        res.resume();
        const next = redirectTo(url, res.headers.location, hops);
        resolve(next.ok ? download(next.url, hops + 1) : next);
      } else if (res.statusCode !== 200) {
        res.resume();
        resolve({ ok: false, reason: `${url} answered HTTP ${res.statusCode}` });
      } else {
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve({ ok: true, body: Buffer.concat(chunks) }));
        res.on("error", (error) => resolve({ ok: false, reason: `${url} did not finish: ${error.message}` }));
      }
    });
    request.on("error", (error) => resolve({ ok: false, reason: `${url} did not finish: ${error.message}` }));
  });
}

async function install() {
  const key = `${process.platform}-${process.arch}`;
  const target = PLATFORM_MAP[key];

  if (!target) {
    console.error(`weeder: no release binary for ${key}`);
    console.error(`Released for: ${Object.keys(PLATFORM_MAP).join(", ")}`);
    console.error(`The release page lists every tarball: ${RELEASES}`);
    process.exit(1);
  }

  const version = require("./package.json").version;
  const url = assetUrl(target, version);
  const binDir = path.join(__dirname, "bin");
  const executable = binaryName(process.platform);

  // Already here from an earlier install: nothing to fetch.
  if (fs.existsSync(path.join(binDir, executable))) {
    return;
  }

  console.log(`weeder: downloading the ${target} artifact`);
  const refuse = (reason) => {
    console.error(`weeder: ${reason}`);
    console.error(`Download the tarball and its .sha256 instead: ${RELEASES}/tag/v${version}`);
    process.exit(1);
  };

  const digest = await download(`${url}.sha256`);
  if (!digest.ok) {
    return refuse(digest.reason);
  }
  const archive = await download(url);
  if (!archive.ok) {
    return refuse(archive.reason);
  }
  const installed = installFrom(archive.body, digest.body.toString("utf8"), assetName(target), binDir, executable);
  if (!installed.ok) {
    return refuse(installed.reason);
  }
  console.log("weeder: installed");
}

module.exports = {
  PLATFORM_MAP,
  MAX_REDIRECTS,
  assetName,
  assetUrl,
  binaryName,
  checkDigest,
  redirectTo,
  installFrom,
};

if (require.main === module) {
  install();
}
