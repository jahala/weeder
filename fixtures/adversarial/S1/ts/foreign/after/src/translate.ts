export function label(name: string): string {
  return name.trim();
}

export function translate(block: { name: string; input: string }): string {
  const todo = block.name === "TodoWrite" ? block.input : null;
  if (todo !== null) {
    return todo;
  }
  const unimplemented = block.input.length === 0;
  return unimplemented ? label(block.name) : block.input;
}
