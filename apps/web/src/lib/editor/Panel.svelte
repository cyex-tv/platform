<script lang="ts">
  // The editor's right panel: a preview of the output, the current tool's settings, turning and
  // size, the file format, and the upload rules as a live checklist with one-click fixes.
  // "Continue to upload" sits at the bottom and works only once every rule passes; the file
  // size is the real encoded file's, encoded after each change settles (or on request for very
  // large animations).
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import EditorIcon from './EditorIcon.svelte';
  import NumField from './NumField.svelte';
  import { AUTO_ENCODE_PIXELS, DRAW_TOOLS, type EditorState } from './state.svelte';
  import { RATIOS } from './geometry';
  import { cropRatio, deleteShape, flipH, flipV, isStretched, outSize, resetCrop, resize, resizeKeep, rotateCcw, rotateCw, setCrop, shapesOn, updateShape, fitTo, type Shape } from './model';
  import { paintOutput, measureText } from './render';
  import { fixLabel } from './rules';
  import { PRESETS, readRecent, saveRecent, withRecent } from './colors';
  import { bytes, count } from '$lib/format';
  import { seconds } from './rules';
  import { total } from './timing';
  import { onMount } from 'svelte';

  let { ed, oncontinue }: { ed: EditorState; oncontinue: () => void } = $props();
  const uid = $props.id();

  const doc = $derived(ed.doc);
  const out = $derived(doc ? outSize(doc) : { w: 1, h: 1 });
  const rules = $derived(ed.rules);
  const failing = $derived(rules.filter((r) => r.state === 'fail').length);
  const shape = $derived(doc && ed.shape !== null ? (doc.shapes.find((s: Shape) => s.id === ed.shape) ?? null) : null);
  const drawing = $derived(DRAW_TOOLS.includes(ed.tool) || !!shape);
  /** What the drawing settings show: the picked drawing's, else the brush's. */
  const kind = $derived<Shape['kind'] | null>(shape?.kind ?? (DRAW_TOOLS.includes(ed.tool) ? (ed.tool as Shape['kind']) : null));
  let recent = $state<string[]>([]);
  onMount(() => (recent = readRecent()));

  // ---- Preview ----
  let preview = $state<HTMLCanvasElement>();
  let work: HTMLCanvasElement | null = null;
  $effect(() => {
    const d = ed.doc;
    const i = ed.current;
    if (!preview || !d) return;
    const f = d.frames[i];
    if (!f) return;
    const o = outSize(d);
    work ??= document.createElement('canvas');
    work.width = o.w;
    work.height = o.h;
    paintOutput(work.getContext('2d')!, d, i, ed.sources[f.src]);
    // At most 128 px tall, as chat shows the 4x size; smaller when the panel is narrow.
    const k = Math.min(1, 128 / o.h, 280 / o.w);
    const dpr = window.devicePixelRatio || 1;
    preview.width = Math.max(1, Math.round(o.w * k * dpr));
    preview.height = Math.max(1, Math.round(o.h * k * dpr));
    preview.style.width = `${Math.max(1, Math.round(o.w * k))}px`;
    preview.style.height = `${Math.max(1, Math.round(o.h * k))}px`;
    const g = preview.getContext('2d')!;
    g.imageSmoothingQuality = 'high';
    g.drawImage(work, 0, 0, preview.width, preview.height);
  });

  // ---- Drawing settings ----
  function setStyle(patch: Partial<Shape> & { size?: number }, label: string) {
    // The brush keeps it for the next drawing; a picked drawing changes now.
    const b = { ...ed.brush };
    for (const k of ['color', 'fill', 'width', 'filled', 'outlined', 'size'] as const) if (patch[k] !== undefined) (b as Record<string, unknown>)[k] = patch[k];
    ed.brush = b;
    if (shape && doc) {
      let p: Partial<Shape> = patch;
      if (shape.kind === 'text' && (patch.size !== undefined || patch.text !== undefined)) {
        const m = measureText(ctx(), patch.text ?? shape.text ?? '', patch.size ?? shape.size ?? 32);
        p = { ...patch, pts: [shape.pts[0], shape.pts[1], m.w, m.h] };
      }
      ed.commit(updateShape(doc, shape.id, p), label);
    }
  }
  let measure: CanvasRenderingContext2D | null = null;
  const ctx = () => (measure ??= document.createElement('canvas').getContext('2d')!);

  function pickColor(which: 'color' | 'fill', c: string, keep = true) {
    setStyle({ [which]: c }, which === 'color' ? 'Colour' : kind === 'text' ? 'Outline colour' : 'Fill colour');
    if (keep) {
      recent = withRecent(recent, c);
      saveRecent(recent);
    }
  }

  const current = $derived(shape ?? ed.brush);
  let textInput = $state<HTMLTextAreaElement>();
  $effect(() => {
    if (ed.focusText) queueMicrotask(() => {
      textInput?.focus();
      textInput?.select();
    });
  });

  function setScope(v: 'all' | 'frame') {
    ed.brush = { ...ed.brush, scope: v };
    if (shape && doc) ed.commit(updateShape(doc, shape.id, { frame: v === 'all' ? null : doc.frames[ed.current].id }), v === 'all' ? 'Show on every frame' : 'Show on this frame only');
  }

  const kindName = (k: Shape['kind']) => ({ pen: 'Pen', line: 'Line', arrow: 'Arrow', rect: 'Rectangle', ellipse: 'Ellipse', text: 'Text' })[k];
  const onFrame = $derived(doc ? shapesOn(doc, doc.frames[ed.current]?.id ?? -1) : []);

  // ---- Crop ----
  function pickRatio(id: string) {
    ed.ratio = id;
    const r = RATIOS.find((x) => x.id === id)?.ratio;
    if (r && doc) ed.commit(cropRatio(doc, r), `Crop ${id}`);
  }
  const setCropField = (k: 'x' | 'y' | 'w' | 'h', v: number) => {
    if (!doc) return;
    const ratio = RATIOS.find((x) => x.id === ed.ratio)?.ratio;
    const r = { ...doc.crop, [k]: v };
    if (ratio && k === 'w') r.h = Math.round(v / ratio);
    if (ratio && k === 'h') r.w = Math.round(v * ratio);
    ed.commit(setCrop(doc, r), 'Crop');
  };

  // ---- Size ----
  function setW(v: number) {
    if (!doc) return;
    ed.commit(ed.keepAspect ? resizeKeep(doc, v, 'w') : resize(doc, v, out.h), ed.keepAspect ? 'Resize' : 'Stretch');
  }
  function setH(v: number) {
    if (!doc) return;
    ed.commit(ed.keepAspect ? resizeKeep(doc, v, 'h') : resize(doc, out.w, v), ed.keepAspect ? 'Resize' : 'Stretch');
  }
  function toggleAspect() {
    ed.keepAspect = !ed.keepAspect;
    // Back to keeping the shape: undo the stretch along the width.
    if (ed.keepAspect && doc && isStretched(doc)) ed.commit({ ...doc, sy: doc.sx }, 'Keep proportions');
  }

  const formats = $derived([
    { value: 'webp' as const, label: 'WebP', disabled: !ed.webp },
    { value: 'gif' as const, label: 'GIF' },
  ]);

  const bigAnimation = $derived(ed.pixels > AUTO_ENCODE_PIXELS);
</script>

{#if doc}
  <div class="panel">
    <div class="body">
      <section class="sec pv" aria-labelledby="pv-{uid}">
        <h3 class="ui-label" id="pv-{uid}">Preview</h3>
        <div class="pvbox"><canvas bind:this={preview} aria-hidden="true"></canvas><span class="sr-only">The emote as it will upload, frame {ed.current + 1}.</span></div>
        <p class="facts ui-mono">{count(out.w)} × {count(out.h)} px · {doc.frames.length === 1 ? 'still' : `${count(doc.frames.length)} frames · ${seconds(total(doc.frames))}`}</p>
      </section>

      {#if ed.tool === 'crop'}
        <section class="sec" aria-labelledby="crop-{uid}">
          <h3 class="ui-label" id="crop-{uid}">Crop</h3>
          <Segmented label="Crop shape" size="s" height={30} options={RATIOS.map((r) => ({ value: r.id, label: r.label }))} value={ed.ratio} onchange={pickRatio} />
          <div class="grid4">
            <NumField label="X" value={doc.crop.x} min={0} onset={(v) => setCropField('x', v)} />
            <NumField label="Y" value={doc.crop.y} min={0} onset={(v) => setCropField('y', v)} />
            <NumField label="W" value={doc.crop.w} onset={(v) => setCropField('w', v)} />
            <NumField label="H" value={doc.crop.h} onset={(v) => setCropField('h', v)} />
          </div>
          <p class="hint">Drag the box or its handles. Shift keeps the shape; arrow keys on the stage move it.</p>
          <button type="button" class="ui-btn secondary small" onclick={() => ed.commit(resetCrop(doc), 'Reset crop')}>Reset crop</button>
        </section>
      {:else if drawing && kind}
        <section class="sec" aria-labelledby="draw-{uid}">
          <h3 class="ui-label" id="draw-{uid}">{shape ? `${kindName(kind)} drawing` : kindName(kind)}</h3>
          {#if kind === 'text'}
            <label class="field">
              <span class="ui-label sub">Text</span>
              <textarea bind:this={textInput} class="ta" rows="2" value={shape?.text ?? ''} placeholder={shape ? '' : 'Click the stage to place text'} disabled={!shape}
                oninput={(e) => shape && doc && ed.preview(updateShape(ed.history.now ?? doc, shape.id, { text: e.currentTarget.value, pts: [shape.pts[0], shape.pts[1], ...Object.values(measureText(ctx(), e.currentTarget.value, shape.size ?? 32))] }))}
                onchange={(e) => setStyle({ text: e.currentTarget.value }, 'Edit text')}></textarea>
            </label>
          {/if}
          <div class="colors">
            <label class="swatch" title="Colour">
              <span class="ui-label sub">{kind === 'text' ? 'Text' : 'Colour'}</span>
              <input type="color" value={current.color} onchange={(e) => pickColor('color', e.currentTarget.value)} />
            </label>
            {#if kind === 'rect' || kind === 'ellipse' || kind === 'text'}
              <label class="swatch" title={kind === 'text' ? 'Outline colour' : 'Fill colour'}>
                <span class="ui-label sub">{kind === 'text' ? 'Outline' : 'Fill'}</span>
                <input type="color" value={current.fill} onchange={(e) => pickColor('fill', e.currentTarget.value)} />
              </label>
            {/if}
          </div>
          <div class="recent" role="group" aria-label="Recent colours">
            {#each [...recent, ...PRESETS.filter((p) => !recent.includes(p))].slice(0, 12) as c (c)}
              <button type="button" class="chip" style:background={c} aria-label="Use {c}" title={c} aria-pressed={current.color === c} onclick={() => pickColor('color', c, false)}></button>
            {/each}
          </div>
          {#if kind === 'text'}
            <label class="range"><span class="ui-label sub">Size</span>
              <input type="range" min="8" max="400" value={shape?.size ?? ed.brush.size} onchange={(e) => setStyle({ size: +e.currentTarget.value }, 'Text size')} />
              <span class="ui-mono v">{Math.round(shape?.size ?? ed.brush.size)}</span></label>
          {/if}
          {#if kind !== 'text' || current.outlined}
            <label class="range"><span class="ui-label sub">{kind === 'text' ? 'Outline' : 'Width'}</span>
              <input type="range" min="1" max="64" value={current.width} onchange={(e) => setStyle({ width: +e.currentTarget.value }, 'Line width')} />
              <span class="ui-mono v">{current.width}</span></label>
          {/if}
          {#if kind === 'rect' || kind === 'ellipse' || kind === 'text'}
            <div class="checks">
              {#if kind !== 'text'}<label class="ui-check"><input type="checkbox" checked={current.filled} onchange={(e) => setStyle({ filled: e.currentTarget.checked }, 'Fill')} /> Fill</label>{/if}
              <label class="ui-check"><input type="checkbox" checked={current.outlined} onchange={(e) => setStyle({ outlined: e.currentTarget.checked }, 'Outline')} /> Outline</label>
            </div>
          {/if}
          <Segmented label="Draw on" size="s" height={30}
            options={[{ value: 'all', label: 'Every frame' }, { value: 'frame', label: 'This frame' }]}
            value={shape ? (shape.frame === null ? 'all' : 'frame') : ed.brush.scope} onchange={(v) => setScope(v as 'all' | 'frame')} />
          {#if shape}
            <button type="button" class="ui-btn danger small" onclick={() => { if (doc && shape) { ed.commit(deleteShape(doc, shape.id), 'Delete drawing'); ed.shape = null; } }}>
              <Icon name="trash" size={14} /> Delete drawing
            </button>
          {/if}
        </section>
      {:else}
        <section class="sec" aria-labelledby="dr-{uid}">
          <h3 class="ui-label" id="dr-{uid}">Drawings on this frame</h3>
          {#if onFrame.length}
            <ul class="shapes">
              {#each onFrame as s (s.id)}
                <li><button type="button" class="row" aria-pressed={ed.shape === s.id} onclick={() => { ed.tool = 'select'; ed.shape = s.id; }}>
                  <span class="dot" style:background={s.color}></span>{kindName(s.kind)}{s.kind === 'text' && s.text ? `: ${s.text.slice(0, 20)}` : ''}<span class="where">{s.frame === null ? 'every frame' : 'this frame'}</span>
                </button></li>
              {/each}
            </ul>
          {:else}
            <p class="hint">None yet. Pick the pen, a shape or text on the left and draw on the stage.</p>
          {/if}
        </section>
      {/if}

      <section class="sec" aria-labelledby="size-{uid}">
        <h3 class="ui-label" id="size-{uid}">Turn and size</h3>
        <div class="turns" role="group" aria-label="Turn and mirror">
          <button type="button" class="tb" aria-label="Rotate left" title="Rotate left ([)" onclick={() => ed.commit(rotateCcw(doc), 'Rotate left')}><EditorIcon name="rotate-ccw" size={16} /></button>
          <button type="button" class="tb" aria-label="Rotate right" title="Rotate right (])" onclick={() => ed.commit(rotateCw(doc), 'Rotate right')}><EditorIcon name="rotate-cw" size={16} /></button>
          <button type="button" class="tb" aria-label="Flip horizontally" title="Flip horizontally" onclick={() => ed.commit(flipH(doc), 'Flip horizontally')}><EditorIcon name="flip-h" size={16} /></button>
          <button type="button" class="tb" aria-label="Flip vertically" title="Flip vertically" onclick={() => ed.commit(flipV(doc), 'Flip vertically')}><EditorIcon name="flip-v" size={16} /></button>
        </div>
        <div class="wh">
          <NumField label="Width" value={out.w} max={8192} onset={setW} />
          <button type="button" class="tb link" aria-label={ed.keepAspect ? 'Keeping proportions; click to stretch' : 'Stretching; click to keep proportions'}
            title={ed.keepAspect ? 'Keep proportions (on)' : 'Stretch (proportions off)'} aria-pressed={ed.keepAspect} onclick={toggleAspect}>
            <EditorIcon name={ed.keepAspect ? 'link' : 'unlink'} size={16} />
          </button>
          <NumField label="Height" value={out.h} max={8192} onset={setH} />
        </div>
        <div class="quick">
          <button type="button" class="ui-btn ghost small" onclick={() => ed.commit(resize(doc, out.w / 2, out.h / 2), 'Half size')}>50%</button>
          <button type="button" class="ui-btn ghost small" onclick={() => ed.commit(resize(doc, doc.crop.w, doc.crop.h), 'Original size')}>100%</button>
          <button type="button" class="ui-btn ghost small" onclick={() => ed.commit(fitTo(doc, 112), 'Fit to 112 px')} title="The usual emote size">112 px</button>
          <button type="button" class="ui-btn ghost small" onclick={() => ed.commit(fitTo(doc, ed.limits.maxSide), `Fit to ${ed.limits.maxSide} px`)}>Fit {count(ed.limits.maxSide)}</button>
        </div>
      </section>

      <section class="sec" aria-labelledby="fmt-{uid}">
        <h3 class="ui-label" id="fmt-{uid}">File</h3>
        <Segmented label="Format" size="s" height={30} options={formats} value={ed.format} onchange={(v) => (ed.format = v)} />
        {#if !ed.webp}<p class="hint">This browser can’t make WebP files, so the emote is saved as a GIF (256 colours a frame).</p>{/if}
        {#if ed.format === 'webp'}
          <label class="range"><span class="ui-label sub">Quality</span>
            <input type="range" min="50" max="100" step="5" value={Math.round(ed.quality * 100)} onchange={(e) => (ed.quality = +e.currentTarget.value / 100)} />
            <span class="ui-mono v">{Math.round(ed.quality * 100)}</span></label>
        {/if}
      </section>

      <section class="sec rules" aria-labelledby="rules-{uid}">
        <h3 class="ui-label" id="rules-{uid}">Upload rules</h3>
        <ul aria-live="polite" aria-relevant="text">
          {#each rules as r (r.id)}
            <li class="rule {r.state}" data-rule={r.id}>
              <span class="ic" aria-hidden="true"><EditorIcon name={r.state === 'pass' ? 'pass' : r.state === 'fail' ? 'fail' : 'wait'} size={16} /></span>
              <span class="rl">
                <span class="name">{r.label}<span class="sr-only">: {r.state === 'pass' ? 'passes' : r.state === 'fail' ? 'too much' : 'checking'}.</span></span>
                <span class="val ui-mono">{#if r.id === 'bytes' && ed.encoding}Encoding {count(ed.encoding.done)} of {count(ed.encoding.total)}…{:else}{r.value}{/if}</span>
                <span class="lim">{r.limit}</span>
              </span>
              {#if r.fix}
                <button type="button" class="ui-btn secondary small fix" onclick={() => ed.fix(r.fix!)}>{fixLabel(r.fix, ed.limits)}</button>
              {:else if r.id === 'bytes' && r.state === 'pending' && !ed.encoding}
                <button type="button" class="ui-btn secondary small fix" onclick={() => void ed.encode()}>Check size</button>
              {/if}
            </li>
          {/each}
        </ul>
        {#if ed.encoding}
          <span class="bar" role="progressbar" aria-label="Encoding" aria-valuemin="0" aria-valuemax={ed.encoding.total} aria-valuenow={ed.encoding.done}>
            <span style:transform="scaleX({Math.max(0.03, ed.encoding.done / Math.max(1, ed.encoding.total))})"></span>
          </span>
        {/if}
        {#if ed.encodeError}<p class="ui-error" role="alert">{ed.encodeError}</p>{/if}
        {#if bigAnimation && !ed.result && !ed.encoding}<p class="hint">A big animation: its size is checked when you ask, so editing stays quick.</p>{/if}
      </section>
    </div>
    <div class="end">
      <p class="state" role="status">
        {#if ed.ready}Every rule passes{#if ed.result}{' · '}<span class="ui-mono">{bytes(ed.result.size)}</span>{/if}.
        {:else if failing}{failing === 1 ? '1 rule doesn’t pass yet.' : `${failing} rules don’t pass yet.`}
        {:else if ed.encoding}Checking the file size…
        {:else}Waiting for the file size.{/if}
      </p>
      {#if failing}
        <div class="fixes">
          {#each rules.filter((r) => r.state === 'fail') as r (r.id)}
            <span class="fl"><span class="ui-mono">{r.label}: {r.value}</span>
              {#if r.fix}<button type="button" class="ui-btn secondary small" onclick={() => ed.fix(r.fix!)}>{fixLabel(r.fix, ed.limits)}</button>{/if}</span>
          {/each}
        </div>
      {/if}
      <button type="button" class="ui-btn primary cont" disabled={!ed.ready || !!ed.encoding} onclick={oncontinue} data-testid="editor-continue">
        Continue to upload <Icon name="arrow-right" size={16} />
      </button>
    </div>
  </div>
{/if}

<style>
  .panel { display: grid; grid-template-rows: minmax(0, 1fr) auto; min-height: 0; min-width: 0; background: var(--surface); border-left: 1px solid var(--border); }
  .body { min-height: 0; overflow: hidden auto; overscroll-behavior: contain; scrollbar-gutter: stable; }
  .sec { display: grid; gap: 10px; padding: 14px 16px; border-bottom: 1px solid var(--border); }
  .sec h3 { margin: 0; }
  .sub { font-size: 11px; }
  .pvbox { display: grid; place-items: center; min-height: 136px; padding: 4px; background: repeating-conic-gradient(var(--checker-a) 0 25%, var(--checker-b) 0 50%) 0 0 / 16px 16px; border: 1px solid var(--border); }
  .pvbox canvas { display: block; max-width: 100%; }
  .facts { margin: 0; font-size: var(--t-fine); color: var(--muted); }
  .hint { margin: 0; font-size: var(--text-hint); color: var(--dim); line-height: 1.45; }
  .grid4 { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 6px; }
  .wh { display: grid; grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr); align-items: end; gap: 6px; }
  .quick, .turns, .checks { display: flex; flex-wrap: wrap; gap: 4px; }
  .tb { display: inline-grid; place-items: center; width: 32px; height: 32px; padding: 0; border: 0; border-radius: var(--radius-btn); background: none; color: var(--muted); cursor: pointer;
    transition: color var(--dur-color), background-color var(--dur-color); }
  .tb[aria-pressed='true'] { color: var(--accent-text); }
  .tb:active { transform: scale(var(--press)); }
  .tb:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .field { display: grid; gap: 4px; }
  .ta { width: 100%; min-height: 56px; padding: 6px 8px; resize: vertical; border: 1px solid var(--field-border, var(--border-strong)); border-radius: var(--radius-btn); background: var(--bg); color: var(--text); font: inherit; font-size: var(--t-small); }
  .ta:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 1px; }
  .colors { display: flex; gap: 12px; }
  .swatch { display: grid; gap: 4px; }
  .swatch input { width: 56px; height: 32px; padding: 2px; border: 1px solid var(--border-strong); border-radius: var(--radius-btn); background: var(--bg); cursor: pointer; }
  .swatch input:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 1px; }
  .recent { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { width: 22px; height: 22px; padding: 0; border: 1px solid var(--border-strong); border-radius: var(--radius-btn); cursor: pointer; }
  .chip[aria-pressed='true'] { box-shadow: 0 0 0 2px var(--bg), 0 0 0 3px var(--text); }
  .chip:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .range { display: grid; grid-template-columns: 64px minmax(0, 1fr) 36px; align-items: center; gap: 8px; }
  .range input { width: 100%; accent-color: var(--accent); }
  .v { font-size: var(--t-fine); color: var(--muted); text-align: right; }
  .shapes { list-style: none; margin: 0; padding: 0; display: grid; }
  .row { display: flex; align-items: center; gap: 8px; width: 100%; min-height: 32px; padding: 0 6px; border: 0; border-radius: var(--radius-btn); background: none; color: var(--text); font: inherit; font-size: var(--t-small); text-align: left; cursor: pointer; }
  .row[aria-pressed='true'] { font-weight: 700; }
  .row:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .dot { width: 10px; height: 10px; border-radius: 50%; box-shadow: 0 0 0 1px var(--border-strong); flex: none; }
  .where { margin-left: auto; font-size: var(--t-fine); color: var(--dim); font-weight: 400; }
  .rules ul { list-style: none; margin: 0; padding: 0; display: grid; }
  .rule { display: grid; grid-template-columns: 20px minmax(0, 1fr) auto; align-items: center; gap: 4px 8px; padding: 7px 0; border-bottom: 1px solid var(--border); }
  .rule:last-child { border-bottom: 0; }
  .ic { display: grid; place-items: center; color: var(--dim); }
  .rule.pass .ic { color: var(--green); }
  .rule.fail .ic { color: var(--red); }
  .rl { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 0 8px; min-width: 0; }
  .name { font-size: var(--t-small); font-weight: 600; }
  .val { font-size: var(--t-fine); text-align: right; color: var(--text); white-space: nowrap; }
  .rule.fail .val { color: var(--red); }
  .lim { grid-column: 1 / -1; font-size: var(--text-hint); color: var(--dim); }
  .fix { grid-column: 2 / -1; justify-self: start; }
  .bar { display: block; height: 3px; border-radius: var(--radius-sm); background: var(--border); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--accent); transform-origin: 0 50%; transition: transform var(--fast) linear; }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .bar span { transition: none; } }
  .end { display: grid; gap: 8px; padding: 12px 16px calc(14px + env(safe-area-inset-bottom, 0px)); border-top: 1px solid var(--border); background: var(--elevated); }
  .state { margin: 0; min-height: 1.45em; font-size: var(--t-fine); color: var(--muted); }
  .cont { width: 100%; min-height: 40px; }
  /* What fails, pinned with its fix, whatever the panel is scrolled to. */
  .fixes { display: grid; gap: 6px; }
  .fl { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-width: 0; font-size: var(--t-fine); color: var(--red); }
  .fl > span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  @media (hover: hover) and (pointer: fine) {
    .tb:hover { color: var(--text); background: var(--surface-hover); }
    .row:hover { background: var(--surface-hover); }
  }
  @media (pointer: coarse) { .tb { width: 44px; height: 44px; } .chip { width: 32px; height: 32px; } .swatch input { height: 40px; } }
  @media (max-width: 1099.98px) { .panel { border-left: 0; border-top: 1px solid var(--border); } .body { overflow: visible; } }
</style>
