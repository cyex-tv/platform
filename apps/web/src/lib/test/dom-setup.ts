// What jsdom lacks and the components use: matchMedia (reduced motion on, so no transitions run;
// phones off), ResizeObserver, the popover API and its :popover-open selector.
import { afterEach } from 'vitest';

window.matchMedia = ((query: string) => ({
  matches: query.includes('prefers-reduced-motion'),
  media: query,
  onchange: null,
  addEventListener() {},
  removeEventListener() {},
  addListener() {},
  removeListener() {},
  dispatchEvent: () => false,
})) as typeof window.matchMedia;

globalThis.ResizeObserver = class {
  observe() {}
  unobserve() {}
  disconnect() {}
};

const OPEN = 'data-popover-open';
HTMLElement.prototype.showPopover = function () {
  this.setAttribute(OPEN, '');
};
HTMLElement.prototype.hidePopover = function () {
  this.removeAttribute(OPEN);
};
const matches = Element.prototype.matches;
Element.prototype.matches = function (selector: string) {
  return selector === ':popover-open' ? this.hasAttribute(OPEN) : matches.call(this, selector);
};
Element.prototype.scrollIntoView = () => {};

afterEach(() => {
  document.body.innerHTML = '';
});
Element.prototype.getAnimations = () => [];
(globalThis as { CSS?: unknown }).CSS ??= { escape: (s: string) => s.replace(/(["\\])/g, '\\$1') };
