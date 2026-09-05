export function ratio(top: number, bottom: number): number {
  return top / bottom;
}

export function total(first: number, second: number): number {
  return first + second;
}

export async function settle(): Promise<string> {
  return "done";
}
