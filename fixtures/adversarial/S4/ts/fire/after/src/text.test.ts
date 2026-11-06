import { test } from "node:test";
import assert from "node:assert/strict";

import { slugify } from "./text";

test("slugify joins the words with a dash", () => {
  assert.equal(slugify(" A b "), "a-b");
});
