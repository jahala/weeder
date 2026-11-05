export function send(payload: string): string {
  try {
    return post(payload);
  } catch (error) {
  }
  return "";
}

export function flush(): Promise<void> {
  return drain().catch(() => {});
}

export function parse(text: string): void {
  try { JSON.parse(text); } catch {}
  try { JSON.parse(text); } catch (error) {}
  try { JSON.parse(text); } catch {} finally { post(text); }
}

function post(payload: string): string {
  return payload.trim();
}

function drain(): Promise<void> {
  return Promise.resolve();
}
