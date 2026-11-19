// One button rule, so a page never mixes the two families:
//   public pages (everything outside the signed-in tool group) use `.btn.btn-s` (40px, 15px type);
//   the tool pages (/me, /admin, in the (tool) layout) use `.ui-btn` (40px, 14px type, compact).
// Both press with scale(--press) and share the 3px corner. A component that renders on both kinds
// of page asks which one it is in; the (tool) layout sets the context.
import { getContext, setContext } from 'svelte';

export type BtnFamily = 'site' | 'tool';
export type BtnKind = 'primary' | 'secondary' | 'ghost' | 'danger';
const KEY = 'btn-family';

/** Called by the (tool) layout. */
export const useToolButtons = () => setContext(KEY, 'tool');
/** Called by a shared component, during init. */
export const btnFamily = (): BtnFamily => (getContext<BtnFamily | undefined>(KEY) === 'tool' ? 'tool' : 'site');

/** The class list for a button of this family. */
export function btnClass(family: BtnFamily, kind: BtnKind = 'secondary'): string {
  if (family === 'tool') return `ui-btn ${kind}`;
  return ['btn btn-s', kind === 'secondary' ? '' : `btn-${kind}`].filter(Boolean).join(' ');
}
