import { describe, expect, it } from "vitest";
import { normalise, parse } from "./parse";

describe("parse", () => {
  it("reads a number", () => {
    expect(parse("42")).toBe(42);
  });
});
