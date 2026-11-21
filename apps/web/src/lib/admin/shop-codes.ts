// What the shop admin's code batches say about themselves, pure so it is tested.

type Batch = {
  codeCount: number;
  usesPerCode: number;
  redemptions: number;
  expiresAt?: string | null;
  disabledAt?: string | null;
};

export type BatchState = 'Active' | 'Turned off' | 'Expired';

/** A batch's state: turned off wins over expired. */
export function batchState(b: Pick<Batch, 'expiresAt' | 'disabledAt'>, now = Date.now()): BatchState {
  if (b.disabledAt) return 'Turned off';
  if (b.expiresAt && Date.parse(b.expiresAt) <= now) return 'Expired';
  return 'Active';
}

/** How many redemptions the batch allows in all, how many were used and how many are left (never below 0). */
export function batchUsage(b: Pick<Batch, 'codeCount' | 'usesPerCode' | 'redemptions'>): { total: number; used: number; left: number } {
  const total = b.codeCount * b.usesPerCode;
  return { total, used: b.redemptions, left: Math.max(0, total - b.redemptions) };
}
