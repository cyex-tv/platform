// What the account hub's error codes say to a person.
import { ApiError } from '$lib/api';

const DELETE: Record<string, string> = {
  confirm_mismatch: 'That isn’t one of your logins. Type your Twitch or Kick login exactly.',
  website_only: 'Delete your account from the website, not from the app.',
  owner_protected: 'This account can’t be deleted because it is protected as an owner account. Another owner has to hand it over first.',
  restricted: 'This account is restricted, so it can’t be deleted right now. Contact support if you need it gone.',
  rate_limited: 'Too many tries. Wait a moment, then try again.',
  signed_out: 'You’re signed out already.',
};

export function deleteMessage(err: unknown): string {
  if (err instanceof ApiError) return DELETE[err.code] ?? err.message;
  return 'Something went wrong. Nothing was deleted.';
}

export function exportMessage(err: unknown): string {
  if (err instanceof ApiError) {
    if (err.code === 'rate_limited') return 'You’ve asked for a copy a few times already. You can ask again in about an hour.';
    return err.message;
  }
  return 'The export didn’t work. Try again in a moment.';
}

const UNLINK: Record<string, string> = {
  restricted: 'Your account is restricted, so you can’t disconnect a sign-in right now.',
  last_identity: 'This is your only connection. Link another platform first, then you can disconnect this one.',
  not_found: 'That platform isn’t connected any more.',
  rate_limited: 'Too many tries. Wait a moment, then try again.',
};

/** The sentence for a failed unlink of a linked Twitch or Kick sign-in. */
export function unlinkMessage(err: unknown): string {
  if (err instanceof ApiError) return UNLINK[err.code] ?? err.message;
  return 'Something went wrong. Nothing was disconnected.';
}
