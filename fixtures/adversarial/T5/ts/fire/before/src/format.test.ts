import { format } from "./format";

describe("format", () => {
  it("pads to the width", () => {
    expect(format("a", 4)).toMatchSnapshot();
  });
});
