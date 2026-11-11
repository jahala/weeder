import { describe, expect, it } from "vitest";
import { endpoint } from "./service";

describe("endpoint", () => {
  it("builds a url from the host", () => {
    expect(endpoint("example.org")).toBe("https://example.org/");
  });
});
