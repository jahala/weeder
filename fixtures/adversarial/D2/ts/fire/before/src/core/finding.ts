export type Finding = {
  rule: string;
  path: string;
};

export function render(finding: Finding): string {
  return `${finding.rule} ${finding.path}`;
}
