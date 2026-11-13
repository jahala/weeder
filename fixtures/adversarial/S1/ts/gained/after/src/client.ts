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
  // FIXME: the trim belongs to the caller. FIXME: so does the pad
  return payload.trim();
}

export function receive(wire: string): string {
  // FIXME: the wire format is still moving
  // TODO: close the socket after the last frame
  return wire;
}
