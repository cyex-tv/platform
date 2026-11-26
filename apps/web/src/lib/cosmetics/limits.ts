// The API's `limits` query, read once per page load and shared: the picture rules and the
// emote caps the upload dialogs check against. A failed read is not kept (the next dialog asks
// again) and the built-in numbers stand in meanwhile.
import { gql } from '$lib/api';
import { PUBLIC_LIMITS } from '$lib/queries';
import type { PublicLimitsQuery } from '$lib/gql/graphql';
import { PICTURE_RULES, rulesFromLimits, type PictureRules } from './upload';

export type PublicLimits = PublicLimitsQuery['limits'];

let pending: Promise<PublicLimits> | null = null;

/** The public limits, asked once (again after a failure). */
export function loadLimits(): Promise<PublicLimits> {
  pending ??= gql(PUBLIC_LIMITS, {})
    .then((r) => r.limits)
    .catch((e) => {
      pending = null;
      throw e;
    });
  return pending;
}

/** The picture rules once the API has answered; the built-in ones if it can't. */
export async function loadPictureRules(): Promise<PictureRules> {
  try {
    return rulesFromLimits(await loadLimits());
  } catch {
    return PICTURE_RULES;
  }
}

/** Forgets the answer (tests). */
export function forgetLimits() {
  pending = null;
}
