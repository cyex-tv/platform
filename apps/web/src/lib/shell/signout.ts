// Signing out, the same from the account menu and the phone menu sheet: on success the session
// is forgotten and the front page reloads its data; on failure you stay put and are told.
import { goto } from '$app/navigation';
import { signOut } from '$lib/api';
import { notify } from '$lib/tool/notices.svelte';
import { forgetLooks } from '$lib/cosmetics/store.svelte';
import { forgetProfile } from '$lib/profile/data.svelte';
import { forgetMe } from '$lib/tool/session.svelte';

export async function leave() {
  try {
    await signOut();
  } catch {
    notify('Signing out didn’t work. Try again in a moment.', true);
    return;
  }
  forgetMe();
  forgetLooks();
  forgetProfile();
  await goto('/', { invalidateAll: true });
}
