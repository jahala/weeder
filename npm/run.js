#!/usr/bin/env node

"use strict";

const { execFileSync } = require("child_process");
const path = require("path");

const isWindows = process.platform === "win32";
const binName = isWindows ? "weeder.exe" : "weeder";
const bin = path.join(__dirname, "bin", binName);

try {
  execFileSync(bin, process.argv.slice(2), { stdio: "inherit" });
} catch (err) {
  // weeder's own codes are the verdict, 0 clean, 2 blocked, 3 could not run , 
  // and the wrapper passes them through untouched.
  if (err.status != null) {
    process.exit(err.status);
  }
  console.error(`weeder: the binary at ${bin} did not run`);
  console.error(err.message);
  // The wrapper could not run weeder, which is weeder's own "could not run" code.
  process.exit(3);
}
