export function send(payload: string): string {
  // TODO: retry once when the queue is full
  return post(payload);
}

function post(payload: string): string {
  // FIXME: the trim belongs to the caller
  return payload.trim();
}

export function receive(): string {
  // XXX: the wire format is still moving
  throw new Error("not implemented");
}

export function drain(): string | null {
  return null;
}
