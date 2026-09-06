import { test } from "node:test";
import assert from "node:assert/strict";

import { parse } from "./parse.ts";

test("splits on commas", () => {
  assert.deepEqual(parse("a,b"), ["a", "b"]);
});
