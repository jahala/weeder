import { describe, expect, it, vi } from "vitest";
import { Editor } from "./editor";

describe("editor", () => {
  it("starts empty", () => {
    const editor = new Editor(() => true);
    expect(editor.submit()).toBe(true);
  });
});
