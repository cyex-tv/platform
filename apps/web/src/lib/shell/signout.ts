// Signing out, the same from the account menu and the phone menu sheet: on success the session
// is forgotten and the front page reloads its data; on failure you stay put and are told.
import { goto } from '$app/navigation';
import { signOut } from '$lib/api';
import { notify } from '$lib/tool/notices.svelte';
import { forgetLooks } from '$lib/cosmetics/store.svelte';
import { forgetProfile } from '$lib/profile/data.svelte';
import { forgetMe } from '$lib/tool/session.svelte';
import { forgetInbox } from '$lib/account/inbox.svelte';
import { forgetPrefs } from '$lib/account/prefs.svelte';
import { dismiss as dismissCheckout } from '$lib/shop/checkout.svelte';

export async function leave() {
  try {
    await signOut();
  } catch {
    notify('Signing out didn’t work. Try again in a moment.', true);
    return;
  }
  await leftSession();
}

/** After the session is gone (signed out, a session revoked, the account deleted): forget it all and go home. */
export async function leftSession() {
  forgetMe();
  forgetLooks();
  forgetProfile();
  forgetInbox();
  forgetPrefs();
  dismissCheckout(); // the next person on this tab must not see the last one's payment
  await goto('/', { invalidateAll: true });
}
