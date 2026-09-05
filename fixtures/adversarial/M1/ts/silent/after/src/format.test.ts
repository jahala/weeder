import { vi } from "vitest";
import { format } from "./format";
import { now } from "./clock";

vi.mock("./clock");

describe("format", () => {
  it("pads to the width", () => {
    expect(format("a", 3)).toBe("a  ");
  });

  it("stamps the run", () => {
    expect(now()).toBeGreaterThan(0);
  });
});
