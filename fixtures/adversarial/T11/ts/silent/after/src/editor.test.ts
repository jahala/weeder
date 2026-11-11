import { describe, expect, it, vi } from "vitest";
import { Editor } from "./editor";

describe("editor", () => {
  it("starts empty", () => {
    const editor = new Editor(() => true);
    expect(editor.submit()).toBe(true);
  });

  it("saves the draft", () => {
    const save = vi.fn().mockReturnValue(true);
    const editor = new Editor(save);
    editor.type("draft");
    editor.submit();
    expect(save).toHaveBeenCalledWith("draft");
  });
});
