function readPlan(text: string): string[] {
  return text.split("\n");
}

function readNode(text: string): string {
  return text.trim();
}

export { readPlan as parsePlan, readNode as parseNode };
