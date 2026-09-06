import { format } from "./format";

const cases = [
  { name: "pads to the width", value: "a", want: "a  " },
  { name: "truncates past the width", value: "abcd", want: "abc" },
  { name: "leaves a value of the width alone", value: "abc", want: "abc" },
];

describe("format", () => {
  for (const { name, value, want } of cases) {
    it(name, () => {
      expect(format(value, 3)).toBe(want);
    });
  }
});
