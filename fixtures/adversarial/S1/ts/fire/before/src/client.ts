export function send(payload: string): string {
  return post(payload);
}

function post(payload: string): string {
  return payload.trim();
}
