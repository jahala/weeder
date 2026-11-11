import assert from "node:assert";
import { describe, expect, it } from "vitest";
import { load, normalise, parse } from "./parse";

function trimsTo(input: string, expected: string) {
  expect(normalise(input)).toBe(expected);
}

describe("parse", () => {
  it("reads a number", () => {
    expect(parse("42")).toBe(42);
  });

  it("refuses an empty string", () => {
    expect(() => parse("")).toThrow("empty input");
  });

  it("refuses an empty string on the way in", async () => {
    await expect(load("")).rejects.toThrow("empty input");
  });

  it("refuses blank space on the way in", async () => {
    await assert.rejects(load("   "), /empty input/);
  });

  it("trims a padded number", () => {
    trimsTo(" 7 ", "7");
  });
});
