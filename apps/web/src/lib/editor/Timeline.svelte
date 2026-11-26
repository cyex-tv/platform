<script lang="ts">
  // The frames under the stage: playback, the frame strip (a listbox: click picks a frame, Shift
  // picks a range, Ctrl or ⌘ adds one; drag to reorder) and what to do with the picked frames.
  // Keys on the strip: ← → move (Shift extends), Home / End, Ctrl + A picks all, Delete removes,
  // Ctrl + D duplicates, Alt + ← → moves the picked frames. Only the frames in view are drawn,
  // so a thousand frames scroll as easily as ten.
  import Icon from '$lib/components/Icon.svelte';
  import EditorIcon from './EditorIcon.svelte';
  import type { EditorState } from './state.svelte';
  import { boomerangFrames, deleteFrames, delayFrames, duplicateFrames, moveFramesTo, reverseFrames, speedFrames, trim, type Doc } from './model';
  import { paintFrame } from './render';
  import { total } from './timing';
  import { seconds } from './rules';
  import { count } from '$lib/format';

  let { ed }: { ed: EditorState } = $props();
  const uid = $props.id();

  const ITEM = 64;
  let strip = $state<HTMLDivElement>();
  let scroll = $state(0);
  let width = $state(0);
  let anchor = 0;

  const doc = $derived(ed.doc);
  const n = $derived(doc?.frames.length ?? 0);
  const first = $derived(Math.max(0, Math.floor(scroll / ITEM) - 2));
  const last = $derived(Math.min(n - 1, Math.ceil((scroll + width) / ITEM) + 2));
  const visible = $derived(n ? Array.from({ length: Math.max(0, last - first + 1) }, (_, k) => first + k) : []);
  const picked = $derived(ed.picked);
  const targets = $derived(ed.targets);
  const many = $derived(targets.length > 1);
  let delayInput = $state('');
  $effect(() => {
    delayInput = doc?.frames[ed.current] ? String(doc.frames[ed.current].delay) : '';
  });

  // Keep the current frame in view.
  $effect(() => {
    const i = ed.current;
    if (!strip) return;
    const left = i * ITEM;
    if (left < strip.scrollLeft) strip.scrollLeft = left - ITEM;
    else if (left + ITEM > strip.scrollLeft + strip.clientWidth) strip.scrollLeft = left + ITEM * 2 - strip.clientWidth;
  });

  function pick(i: number, e: { shiftKey: boolean; ctrlKey: boolean; metaKey: boolean }) {
    ed.show(i);
    if (e.shiftKey) {
      const [a, b] = [Math.min(anchor, i), Math.max(anchor, i)];
      ed.picked = new Set(Array.from({ length: b - a + 1 }, (_, k) => a + k));
    } else if (e.ctrlKey || e.metaKey) {
      const next = new Set(picked.size ? picked : [ed.current]);
      if (next.has(i) && next.size > 1) next.delete(i);
      else next.add(i);
      ed.picked = next;
      anchor = i;
    } else {
      ed.picked = new Set();
      anchor = i;
    }
  }

  const plural = (k: number, one: string) => (k === 1 ? `1 ${one}` : `${count(k)} ${one}s`);

  function act(next: Doc, label: string, current = ed.current) {
    ed.commit(next, label);
    ed.picked = new Set();
    ed.show(current);
  }

  const remove = () => doc && n > 1 && act(deleteFrames(doc, targets), `Delete ${plural(Math.min(targets.length, n - 1), 'frame')}`, Math.min(targets[0], n - 1 - Math.min(targets.length, n - 1)));
  const duplicate = () => doc && act(duplicateFrames(doc, targets), `Duplicate ${plural(targets.length, 'frame')}`, targets.at(-1)! + targets.length);
  const keep = () => doc && many && act(trim(doc, targets[0], targets.at(-1)!), `Trim to frames ${targets[0] + 1} to ${targets.at(-1)! + 1}`, 0);
  const reverse = () => doc && act(reverseFrames(doc), 'Reverse');
  const boomerang = () => doc && act(boomerangFrames(doc), 'Boomerang');

  function setDelay(all: boolean) {
    const v = Math.round(Number(delayInput.replace(',', '.')));
    if (!doc || !Number.isFinite(v) || v < 1) {
      delayInput = doc?.frames[ed.current] ? String(doc.frames[ed.current].delay) : '';
      return;
    }
    ed.commit(delayFrames(doc, v, all ? undefined : targets), all ? `Delay ${v} ms for every frame` : `Delay ${v} ms`);
  }

  function speed(e: Event & { currentTarget: HTMLSelectElement }) {
    const k = Number(e.currentTarget.value);
    e.currentTarget.value = '';
    if (doc && k > 0) ed.commit(speedFrames(doc, k), `Speed ×${String(k).replace('.', ',')}`);
  }

  function move(by: number) {
    if (!doc) return;
    const t = targets;
    const to = by < 0 ? Math.max(0, t[0] - 1) : Math.min(n, t.at(-1)! + 2);
    if ((by < 0 && t[0] === 0) || (by > 0 && t.at(-1) === n - 1)) return;
    const moved = moveFramesTo(doc, t, to);
    const start = by < 0 ? t[0] - 1 : t[0] + 1;
    ed.commit(moved, t.length > 1 ? 'Move frames' : 'Move frame');
    ed.show(ed.current + by);
    ed.picked = t.length > 1 ? new Set(t.map((_, k) => start + k)) : new Set();
  }

  function keydown(e: KeyboardEvent) {
    if (!doc) return;
    const mod = e.ctrlKey || e.metaKey;
    let to: number | null = null;
    if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
      const by = e.key === 'ArrowLeft' ? -1 : 1;
      if (e.altKey) {
        e.preventDefault();
        return move(by);
      }
      to = Math.max(0, Math.min(n - 1, ed.current + by));
    } else if (e.key === 'Home') to = 0;
    else if (e.key === 'End') to = n - 1;
    else if (mod && e.key.toLowerCase() === 'a') {
      e.preventDefault();
      ed.picked = new Set(doc.frames.map((_, i) => i));
      return;
    } else if (mod && e.key.toLowerCase() === 'd') {
      e.preventDefault();
      return duplicate();
    } else if (e.key === 'Delete' || e.key === 'Backspace') {
      e.preventDefault();
      return remove();
    }
    if (to === null) return;
    e.preventDefault();
    if (e.shiftKey) {
      ed.show(to);
      const [a, b] = [Math.min(anchor, to), Math.max(anchor, to)];
      ed.picked = new Set(Array.from({ length: b - a + 1 }, (_, k) => a + k));
    } else pick(to, { shiftKey: false, ctrlKey: false, metaKey: false });
  }

  // ---- Drag to reorder ----
  let press: { i: number; x: number; id: number } | null = null;
  let dropAt = $state<number | null>(null);

  function itemDown(e: PointerEvent, i: number) {
    if (e.button !== 0) return;
    press = { i, x: e.clientX, id: e.pointerId };
  }
  function stripMove(e: PointerEvent) {
    if (!press || !strip) return;
    if (dropAt === null && Math.abs(e.clientX - press.x) < 6) return;
    if (dropAt === null) strip.setPointerCapture(press.id);
    const r = strip.getBoundingClientRect();
    dropAt = Math.max(0, Math.min(n, Math.round((e.clientX - r.left + strip.scrollLeft) / ITEM)));
  }
  function stripUp() {
    const p = press;
    const at = dropAt;
    press = null;
    dropAt = null;
    if (!p || at === null || !doc) return;
    const moving = picked.has(p.i) ? [...picked].sort((a, b) => a - b) : [p.i];
    if (at >= moving[0] && at <= moving.at(-1)! + 1 && moving.length === moving.at(-1)! - moving[0] + 1) return;
    const before = moving.filter((i) => i < at).length;
    ed.commit(moveFramesTo(doc, moving, at), moving.length > 1 ? 'Move frames' : 'Move frame');
    ed.picked = new Set();
    ed.show(at - before);
  }

  // A thumbnail: the frame as it will upload (cropped), fitted in the box.
  function thumb(node: HTMLCanvasElement, args: { d: Doc; i: number }) {
    const paint = ({ d, i }: { d: Doc; i: number }) => {
      const f = d.frames[i];
      const src = f && ed.sources[f.src];
      const g = node.getContext('2d');
      if (!g || !src) return;
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      const W = 56 * dpr;
      const H = 40 * dpr;
      node.width = W;
      node.height = H;
      const c = d.crop;
      const k = Math.min(W / c.w, H / c.h);
      g.clearRect(0, 0, W, H);
      g.imageSmoothingQuality = 'medium';
      paintFrame(g, d, i, src, [k, 0, 0, k, (W - c.w * k) / 2 - c.x * k, (H - c.h * k) / 2 - c.y * k]);
      // Only the crop shows.
      g.globalCompositeOperation = 'destination-in';
      g.setTransform(1, 0, 0, 1, 0, 0);
      g.fillRect((W - c.w * k) / 2, (H - c.h * k) / 2, c.w * k, c.h * k);
      g.globalCompositeOperation = 'source-over';
    };
    paint(args);
    return { update: paint };
  }

  const speeds = [0.25, 0.5, 0.75, 1.25, 1.5, 2, 3, 4];
</script>

{#if doc}
  <div class="tl">
    <div class="bar">
      <div class="group">
        <button type="button" class="tb" aria-label="Previous frame" title="Previous frame (,)" onclick={() => ed.show((ed.current - 1 + n) % n)} disabled={n < 2}><EditorIcon name="skip-back" size={16} /></button>
        <button type="button" class="tb" aria-label={ed.playing ? 'Pause' : 'Play'} title="{ed.playing ? 'Pause' : 'Play'} (K)" onclick={() => ed.toggle()} disabled={n < 2}>
          <Icon name={ed.playing ? 'pause' : 'play'} size={16} />
        </button>
        <button type="button" class="tb" aria-label="Next frame" title="Next frame (.)" onclick={() => ed.show((ed.current + 1) % n)} disabled={n < 2}><EditorIcon name="skip-forward" size={16} /></button>
        <span class="pos ui-mono" aria-live="off">{count(ed.current + 1)} / {count(n)}</span>
        <span class="len ui-mono" title="One loop">{seconds(total(doc.frames))}</span>
        <button type="button" class="tb" aria-label="Onion skin" title="Onion skin: show the frame before, faint" aria-pressed={ed.onion} onclick={() => (ed.onion = !ed.onion)} disabled={n < 2}><EditorIcon name="onion" size={16} /></button>
      </div>
      <div class="group acts">
        <label class="delay">
          <span class="ui-label">Delay</span>
          <input class="num" inputmode="numeric" bind:value={delayInput} aria-describedby="delay-{uid}"
            onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); setDelay(false); } }} onchange={() => setDelay(false)} />
          <span class="unit" id="delay-{uid}">ms{many ? ` · ${count(targets.length)} frames` : ''}</span>
        </label>
        <button type="button" class="ui-btn ghost small" onclick={() => setDelay(true)} title="Use this delay for every frame">All frames</button>
        <select class="speed" aria-label="Change speed" onchange={speed} disabled={n < 2}>
          <option value="">Speed…</option>
          {#each speeds as s (s)}<option value={s}>{s < 1 ? `${s * 100}% (slower)` : `${s * 100}% (faster)`}</option>{/each}
        </select>
        <button type="button" class="tb" aria-label="Duplicate {many ? 'frames' : 'frame'}" title="Duplicate (Ctrl+D)" onclick={duplicate}><EditorIcon name="duplicate" size={16} /></button>
        <button type="button" class="tb" aria-label="Delete {many ? 'frames' : 'frame'}" title="Delete (Del)" onclick={remove} disabled={n < 2 || targets.length >= n}><Icon name="trash" size={16} /></button>
        <button type="button" class="tb" aria-label="Keep only the picked frames" title="Trim to the picked frames (Shift-click a range first)" onclick={keep} disabled={!many}><EditorIcon name="scissors" size={16} /></button>
        <button type="button" class="tb" aria-label="Reverse" title="Reverse" onclick={reverse} disabled={n < 2}><EditorIcon name="reverse" size={16} /></button>
        <button type="button" class="tb" aria-label="Boomerang" title="Boomerang: forward, then back" onclick={boomerang} disabled={n < 3}><EditorIcon name="boomerang" size={16} /></button>
      </div>
    </div>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="strip"
      bind:this={strip}
      bind:clientWidth={width}
      onscroll={() => (scroll = strip?.scrollLeft ?? 0)}
      role="listbox"
      aria-label="Frames"
      aria-multiselectable="true"
      aria-orientation="horizontal"
      aria-activedescendant="f-{uid}-{ed.current}"
      tabindex="0"
      onkeydown={keydown}
      onpointermove={stripMove}
      onpointerup={stripUp}
      onpointercancel={() => {
        press = null;
        dropAt = null;
      }}
    >
      <div class="track" style:width="{n * ITEM}px">
        {#each visible as i (doc.frames[i]?.id ?? i)}
          {@const f = doc.frames[i]}
          {#if f}
            <div
              class="item"
              id="f-{uid}-{i}"
              role="option"
              tabindex="-1"
              aria-selected={i === ed.current || picked.has(i)}
              aria-label="Frame {i + 1}, {f.delay} ms"
              class:cur={i === ed.current}
              class:pick={picked.has(i)}
              style:left="{i * ITEM}px"
              onpointerdown={(e) => itemDown(e, i)}
              onclick={(e) => pick(i, e)}
              onkeydown={() => {}}
            >
              <canvas class="th" use:thumb={{ d: doc, i }} aria-hidden="true"></canvas>
              <span class="meta ui-mono">{i + 1}<span class="d">{f.delay}</span></span>
            </div>
          {/if}
        {/each}
        {#if dropAt !== null}<span class="drop" style:left="{dropAt * ITEM - 2}px" aria-hidden="true"></span>{/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .tl { display: grid; grid-template-rows: auto auto; min-width: 0; border-top: 1px solid var(--border); background: var(--surface); }
  .bar { display: flex; align-items: center; justify-content: space-between; gap: 4px 12px; flex-wrap: wrap; min-width: 0; padding: 4px 8px; border-bottom: 1px solid var(--border); }
  .group { display: flex; align-items: center; gap: 2px; min-width: 0; }
  .acts { flex-wrap: wrap; justify-content: flex-end; gap: 4px; }
  .tb { display: inline-grid; place-items: center; width: 32px; height: 32px; padding: 0; border: 0; border-radius: var(--radius-btn); background: none; color: var(--muted);
    cursor: pointer; transition: color var(--dur-color), background-color var(--dur-color); }
  .tb[aria-pressed='true'] { color: var(--accent-text); }
  .tb:disabled { opacity: .4; cursor: not-allowed; }
  .tb:active:not(:disabled) { transform: scale(var(--press)); }
  .tb:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .pos, .len { padding: 0 8px; font-size: var(--t-fine); color: var(--muted); white-space: nowrap; }
  .pos { min-width: 92px; text-align: center; color: var(--text); }
  .len { border-left: 1px solid var(--border); }
  .delay { display: inline-flex; align-items: center; gap: 6px; margin-right: 2px; }
  .num { width: 62px; height: 30px; padding: 0 8px; border: 1px solid var(--field-border, var(--border-strong)); border-radius: var(--radius-btn); background: var(--bg); color: var(--text);
    font: inherit; font-size: var(--t-fine); font-weight: 500; font-variant-numeric: tabular-nums; text-align: right; }
  .num:focus-visible, .speed:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 1px; }
  .unit { font-size: var(--t-fine); color: var(--dim); white-space: nowrap; }
  .speed { height: 30px; padding: 0 6px; border: 1px solid var(--field-border, var(--border-strong)); border-radius: var(--radius-btn); background: var(--bg); color: var(--text); font: inherit; font-size: var(--t-fine); }
  .strip { position: relative; height: 72px; overflow: auto hidden; overscroll-behavior: contain; scrollbar-width: thin; outline: none; touch-action: pan-x; }
  .strip:focus-visible { box-shadow: inset 0 0 0 2px var(--focus-ring); }
  .track { position: relative; height: 100%; }
  .item { position: absolute; top: 6px; width: 60px; height: 60px; display: grid; grid-template-rows: 42px auto; justify-items: center; gap: 2px; padding: 0 2px;
    border-radius: var(--radius-btn); cursor: pointer; user-select: none; -webkit-user-select: none; }
  .th { width: 56px; height: 40px; border-radius: 2px; background: repeating-conic-gradient(var(--checker-a) 0 25%, var(--checker-b) 0 50%) 0 0 / 10px 10px; box-shadow: 0 0 0 1px var(--border); }
  .item.pick .th { box-shadow: 0 0 0 2px var(--accent-border); }
  .item.cur .th { box-shadow: 0 0 0 2px var(--accent); }
  .meta { display: flex; gap: 6px; font-size: 11px; line-height: 14px; color: var(--dim); }
  .item.cur .meta { color: var(--text); font-weight: 600; }
  .d { color: var(--dim); font-weight: 400; }
  .drop { position: absolute; top: 4px; bottom: 4px; width: 3px; background: var(--accent); border-radius: 2px; }
  @media (hover: hover) and (pointer: fine) {
    .tb:hover:not(:disabled) { color: var(--text); background: var(--surface-hover); }
    .item:hover .th { box-shadow: 0 0 0 1px var(--border-strong); }
    .item.cur:hover .th { box-shadow: 0 0 0 2px var(--accent); }
  }
  @media (pointer: coarse) { .tb { width: 40px; height: 40px; } .num, .speed { height: 40px; font-size: 16px; } }
</style>
