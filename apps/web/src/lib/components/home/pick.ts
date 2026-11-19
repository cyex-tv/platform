// Which browser the page is open in, for the /download page's card order. Pure, so it's tested without a browser.
/** Which install card leads: the browser the page is open in, when it can take the extension. */
export type Browser = 'chrome' | 'firefox' | 'other';

export function browserOf(ua: string): Browser {
  if (/iPhone|iPad|iPod/.test(ua)) return 'other'; // no extensions on iOS browsers
  if (/Firefox\//.test(ua)) return 'firefox';
  if (/Android/.test(ua)) return 'other'; // Chrome on Android takes no extensions
  if (/Chrome\/|Chromium\//.test(ua)) return 'chrome'; // Edge, Brave, Opera and Vivaldi too
  return 'other';
}
