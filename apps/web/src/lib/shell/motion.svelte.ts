// Appearance → Reduced motion, per device: System (follow the OS), On (reduce everywhere), Off
// (keep motion even when the OS asks for less). Saved in localStorage (cyexMotion). app.html sets
// <html data-motion> before the first paint and makes matchMedia('(prefers-reduced-motion: …)')
// answer by the setting, so every script that reads the query follows it; app.css turns
// transitions and animations off under data-motion="reduce".
import { MOTION_KEY, readMotion, type MotionPref } from '$lib/account/settings';

export const motion = $state<{ pref: MotionPref }>({ pref: 'system' });

/** Reads what the inline script applied (call once mounted). */
export function syncMotion() {
  try {
    motion.pref = readMotion(localStorage.getItem(MOTION_KEY));
  } catch {
    motion.pref = readMotion(document.documentElement.dataset.motion);
  }
}

export function setMotion(pref: MotionPref) {
  motion.pref = pref;
  const root = document.documentElement;
  if (pref === 'system') delete root.dataset.motion;
  else root.dataset.motion = pref;
  try {
    if (pref === 'system') localStorage.removeItem(MOTION_KEY);
    else localStorage.setItem(MOTION_KEY, pref);
  } catch {}
  dispatchEvent(new Event('cyexmotion'));
}
