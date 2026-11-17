// The event socket (apps/events), for the reference's WS block. It has no OpenAPI entry, so its
// frames are written here; the event types come from the GraphQL schema's EventType.
import { EX } from './example';
import type { GqlOp, GqlReference } from './graphql';
import { baseUrl } from './rest';

export const socketUrl = `${baseUrl.replace(/^http/, 'ws')}/v1/events`;

export const socketFrames: [dir: string, op: string, d: string][] = [
  ['server → client', 'hello', '`session_id`, `heartbeat_interval` (ms), `max_subscriptions`.'],
  ['client → server', 'subscribe', '`topics`: the topics to add.'],
  ['client → server', 'unsubscribe', '`topics`: the topics to drop.'],
  ['server → client', 'ack', '`op`, `topics`, and an `error` code for each topic that was refused.'],
  ['server → client', 'dispatch', '`id` (the event ID), `type`, `data`.'],
  ['both', 'heartbeat', 'Empty. Every 20 s from both sides; a client silent for 60 s is closed.'],
  ['client → server', 'resume', '`session_id`, `last_event_id`.'],
  ['server → client', 'reconnect', 'The server is restarting: reconnect and resume.'],
  ['server → client', 'error', '`code`, `message`; then the connection closes.'],
];

export const socketTopics: [topic: string, d: string][] = [
  ['channel:<platform>:<id>', 'A channel: its active set, and the emotes in it. `platform` is `twitch` or `kick`.'],
  ['set:<id>', 'One set.'],
  ['user:<platform>:<id>', 'A user\'s personal set.'],
];

/** `SET_EMOTE_ADDED` is written `set.emote_added` on the socket. */
export function eventTypes(gql: GqlReference) {
  return (gql.types.find((t) => t.name === 'EventType')?.members ?? []).map((m) => ({
    wire: m.name.toLowerCase().replace('_', '.'),
    description: m.description,
  }));
}

export const socketClient = `const ws = new WebSocket('${socketUrl}');
let sessionId = null;
let lastEventId = null;

const send = (op, d = {}) => ws.send(JSON.stringify({ op, d }));

ws.onmessage = ({ data }) => {
  const { op, d } = JSON.parse(data);
  if (op === 'hello') {
    if (sessionId && lastEventId) {
      // A reconnect: replay what was missed on the old topics.
      send('resume', { session_id: sessionId, last_event_id: lastEventId });
    } else {
      send('subscribe', { topics: ['channel:twitch:${EX.twitchId}', 'set:${EX.setId}'] });
    }
    sessionId = d.session_id;
  } else if (op === 'dispatch') {
    lastEventId = d.id;
    console.log(d.type, d.data);
  } else if (op === 'heartbeat') {
    send('heartbeat');
  } else if (op === 'reconnect') {
    ws.close();
  }
};`;

export const socketExample = `← { "op": "hello", "d": { "session_id": "s_8f2c41d0", "heartbeat_interval": 20000, "max_subscriptions": 500 } }
→ { "op": "subscribe", "d": { "topics": ["channel:twitch:${EX.twitchId}", "set:${EX.setId}"] } }
← { "op": "ack", "d": { "op": "subscribe", "topics": ["channel:twitch:${EX.twitchId}", "set:${EX.setId}"] } }
← { "op": "dispatch", "d": { "id": "1024", "type": "set.emote_added", "data": { "set_id": "${EX.setId}", "emote_id": "${EX.emoteId}", "name": "${EX.emoteName}" } } }
→ { "op": "resume", "d": { "session_id": "s_8f2c41d0", "last_event_id": "1024" } }`;

/** graphql-transport-ws frames for a subscription, as a client sends and receives them. */
export function subscriptionFrames(op: GqlOp, socket: string): string {
  const payload = { query: op.document.replace(/\s+/g, ' '), variables: op.variables ? JSON.parse(op.variables) : {} };
  const next = { type: 'next', id: '1', payload: JSON.parse(op.response) };
  return [
    `// ${socket}, subprotocol "graphql-transport-ws"`,
    `→ ${JSON.stringify({ type: 'connection_init' })}`,
    `← ${JSON.stringify({ type: 'connection_ack' })}`,
    `→ ${JSON.stringify({ type: 'subscribe', id: '1', payload })}`,
    `← ${JSON.stringify(next)}`,
  ].join('\n');
}
