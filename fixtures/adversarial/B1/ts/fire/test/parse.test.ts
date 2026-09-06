import { test } from "node:test";
import assert from "node:assert/strict";

import { parse } from "./parse.ts";

test("splits on commas", () => {
  assert.deepEqual(parse("a,b"), ["a", "b"]);
});

test("keeps an empty field", () => {
  assert.deepEqual(parse("a,,b"), ["a", "", "b"]);
});

test("keeps a single field whole", () => {
  assert.deepEqual(parse("a"), ["a"]);
});
