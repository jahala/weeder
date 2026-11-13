// TODO(#2013): replace this lock with an asynchronous communication channel
let lock = false;

export function send(payload: string): string {
  lock = true;
  const sent = post(payload);
  // TODO: retry once when the queue is full
  lock = false;
  return sent;
}

function post(payload: string): string {
  return payload.trim();
}
