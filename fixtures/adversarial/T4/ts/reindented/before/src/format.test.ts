import { ratio } from "./format";

it("divides to four places", () => {
  expect(ratio(2, 3)).toBeCloseTo(0.6667, 4);
  expect(ratio(1, 3)).toBeCloseTo(0.3333, 4);
});
