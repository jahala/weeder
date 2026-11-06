import { contrast } from "./contrast";

const themes = [
  ["light", "a light page"],
  ["dark", "a dark page"],
] as const;

const textPairs: [string, string][] = [
  ["text", "paper"],
  ["muted", "paper"],
];

const edgePairs = [{ mark: "edge", ground: "paper" }];

const markPairs = [...edgePairs, { mark: "focus", ground: "paper" }];

describe("contrast", () => {
  describe.each(themes.map(([theme, page]) => [page, theme] as const))("on %s", (page, theme) => {
    it.each(textPairs)(`text %s on %s reads on ${page}`, (fg, bg) => {
      expect(contrast(theme, fg, bg)).toBeGreaterThanOrEqual(4.5);
    });

    it.each(markPairs)(`mark $mark on $ground shows on ${page}`, ({ mark, ground }) => {
      expect(contrast(theme, mark, ground)).toBeGreaterThanOrEqual(3);
    });
  });
});
