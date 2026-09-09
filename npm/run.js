#!/usr/bin/env node

"use strict";

const { execFileSync } = require("child_process");
const path = require("path");

// The installer names the executable it unpacked; the runner asks it rather
// than spelling the name a second time.
const { binaryName } = require("./install.js");

const bin = path.join(__dirname, "bin", binaryName(process.platform));

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
