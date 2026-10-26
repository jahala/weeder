import { format } from "./format";
import padded from "./__fixtures__/padded.json";

describe("format", () => {
  it("keeps a value already at the width", () => {
    expect(format("abcd", 4)).toBe("abcd");
  });

  it("pads a short value with dots", () => {
    expect(format(padded.value, padded.width)).toBe(padded.padded);
  });
});
