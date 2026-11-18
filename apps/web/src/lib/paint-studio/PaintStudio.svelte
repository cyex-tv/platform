<script lang="ts">
  // The paint studio: make a cyex.tv name paint, gradient or animated, see it on names and in
  // chat, and save it to cyex.tv. Works without a backend too (import, export, download).
  //   <PaintStudio backend={apiBackend()} paintId={id} onsaved={(s) => goto(...)} />
  import { onDestroy, onMount, untrack } from 'svelte';
  import './studio.css';
  import type { AnimDoc, Engine } from './engine/index.js';
  import { cachePictures, recallProject, rememberProject } from './anim';
  import type { SavedPaint, StudioBackend, UploadState } from './backend';
  import { paintCss, paintStyleText } from './css';
  import { blankPaint, checkPicture, clonePaint, fmtBytes, LIMITS, validatePaint, type Issue, type StudioPaint } from './format';
  import { engineShadows, exportCyex, exportPaint, importPaint, projectDoc, toEngineShadows } from './interop';
  import { createRenderer, Cancelled, type Format } from './render';
  import { EXT, MIME } from './render-job';
  import AnimEditor from './ui/AnimEditor.svelte';
  import Colour from './ui/Colour.svelte';
  import Glyph from './ui/Glyph.svelte';
  import Preview from './ui/Preview.svelte';
  import Seg from './ui/Seg.svelte';
  import Shadows from './ui/Shadows.svelte';
  import StaticEditor from './ui/StaticEditor.svelte';

  type Props = {
    backend?: StudioBackend | null;
    /** Open this cyex.tv paint on load. */
    paintId?: string | null;
    onsaved?: (saved: SavedPaint) => void;
  };
  let { backend = null, paintId = null, onsaved }: Props = $props();

  type Mode = 'static' | 'animated';
  type Source = 'build' | 'file' | 'existing';

  let paint = $state<StudioPaint>(blankPaint());
  let savedId = $state<string | null>(null);
  let mode = $state<Mode>('static');
  let gradientKind = $state<'LINEAR_GRADIENT' | 'RADIAL_GRADIENT'>('LINEAR_GRADIENT');
  let source = $state<Source>('build');
  let PL = $state<Engine | null>(null);
  let doc = $state<AnimDoc | null>(null);
  let previewName = $state('');
  let reduced = $state(false);
  let playing = $state(true);
  let status = $state<{ text: string; bad?: boolean; progress?: number } | null>(null);
  let busy = $state(false);
  let warnings = $state<string[]>([]);

  // animated previews: the latest render (APNG) and its first frame
  let stillUrl = $state<string | null>(null);
  let animUrl = $state<string | null>(null);
  let renderProgress = $state<number | null>(null);
  let renderError = $state<string | null>(null);
  // the saved picture of an existing image paint, and a picked file
  let existingStill = $state<string | null>(null);
  let picked = $state<{ file: File; url: string; still: string | null; width: number; height: number } | null>(null);
  let pickError = $state<string[]>([]);
  let format = $state<Format>('webp');
  let made = $state<{ blob: Blob; url: string; key: string } | null>(null);

  const preview = createRenderer();
  const output = createRenderer();
  const urls = new Set<string>();
  const keep = (blob: Blob) => {
    const u = URL.createObjectURL(blob);
    urls.add(u);
    return u;
  };
  const drop = (u: string | null) => {
    if (u && urls.has(u)) {
      URL.revokeObjectURL(u);
      urls.delete(u);
    }
  };

  onMount(() => {
    const mq = matchMedia('(prefers-reduced-motion: reduce)');
    reduced = mq.matches;
    playing = !mq.matches;
    const on = () => {
      reduced = mq.matches;
      if (mq.matches) playing = false;
    };
    mq.addEventListener('change', on);
    if (backend && paintId) void openById(paintId);
    return () => mq.removeEventListener('change', on);
  });
  onDestroy(() => {
    preview.dispose();
    output.dispose();
    for (const u of urls) URL.revokeObjectURL(u);
  });

  async function engine(): Promise<Engine> {
    if (!PL) PL = (await import('./engine/index.js')).default;
    return PL;
  }

  // ---- mode ----

  async function setMode(m: Mode) {
    if (m === mode) return;
    if (m === 'animated') {
      if (paint.kind !== 'IMAGE') gradientKind = paint.kind;
      paint.kind = 'IMAGE';
      const E = await engine();
      if (!doc) doc = E.Engine.sanitizeDoc(E.ANIM_PRESETS[0]()).doc;
    } else {
      paint.kind = gradientKind;
      if (paint.stops.length < 2) paint.stops = blankPaint().stops;
    }
    mode = m;
  }

  // ---- the paint being checked: an animated build gets its picture when it's saved ----

  const issues = $derived.by<Issue[]>(() => {
    const p = paint.kind === 'IMAGE' && source !== 'existing' ? { ...paint, image: { id: 'pending', url: 'pending' } } : paint;
    const out = validatePaint(p);
    if (mode === 'animated' && source === 'build' && doc && !doc.layers.some((l) => l.visible)) out.push({ field: 'image', message: 'Add a layer (or start from a preset) to make the picture.' });
    if (mode === 'animated' && source === 'existing' && !paint.image?.id) out.push({ field: 'image', message: 'This picture isn’t on cyex.tv yet: upload the file or build it here.' });
    if (mode === 'animated' && source === 'file' && !picked) out.push({ field: 'image', message: 'Choose a picture file.' });
    if (mode === 'animated' && source === 'file' && pickError.length) out.push(...pickError.map((message) => ({ field: 'image' as const, message })));
    return out;
  });
  const nameIssue = $derived(issues.find((i) => i.field === 'name')?.message ?? null);
  const nameLength = $derived([...paint.name.trim()].length);

  // ---- live animated preview (in the worker, debounced) ----

  $effect(() => {
    if (mode !== 'animated' || source !== 'build' || !doc || !PL) return;
    const snapshot = JSON.stringify(doc);
    const timer = setTimeout(() => untrack(() => void renderPreview(JSON.parse(snapshot))), 180);
    return () => clearTimeout(timer);
  });

  async function renderPreview(d: AnimDoc) {
    renderError = null;
    renderProgress = 0;
    try {
      const r = await preview.render(d, {
        format: 'apng',
        onStill: (b) => {
          drop(stillUrl);
          stillUrl = keep(b);
        },
        onProgress: (done, total) => (renderProgress = done / total),
      });
      drop(animUrl);
      animUrl = keep(r.blob);
      renderProgress = null;
    } catch (e) {
      if (e instanceof Cancelled) return;
      renderProgress = null;
      renderError = (e as Error).message || 'Couldn’t render the preview.';
    }
  }

  const previewImage = $derived.by(() => {
    if (mode !== 'animated') return null;
    if (source === 'build') return playing ? (animUrl ?? stillUrl) : stillUrl;
    if (source === 'file') return picked ? (playing ? picked.url : (picked.still ?? picked.url)) : null;
    return playing ? paint.image?.url ?? null : (existingStill ?? paint.image?.url ?? null);
  });

  // ---- picture file ----

  async function pickFile(file: File | undefined) {
    pickError = [];
    if (!file) return;
    if (!['image/gif', 'image/png', 'image/apng', 'image/webp'].includes(file.type)) {
      pickError = ['Choose a GIF, PNG, APNG or WebP file.'];
      return;
    }
    try {
      const bmp = await createImageBitmap(file);
      const cv = document.createElement('canvas');
      cv.width = bmp.width;
      cv.height = bmp.height;
      cv.getContext('2d')!.drawImage(bmp, 0, 0);
      const stillBlob = await new Promise<Blob | null>((r) => cv.toBlob(r, 'image/png'));
      if (picked) {
        drop(picked.url);
        drop(picked.still);
      }
      picked = { file, url: keep(file), still: stillBlob ? keep(stillBlob) : null, width: bmp.width, height: bmp.height };
      pickError = checkPicture({ bytes: file.size, width: bmp.width, height: bmp.height });
      bmp.close();
    } catch {
      pickError = ['Couldn’t read that picture. Try another file.'];
    }
  }

  // ---- making the output file ----

  async function makeFile(): Promise<{ blob: Blob; url: string }> {
    if (!doc) throw new Error('Nothing to render.');
    const key = JSON.stringify(doc) + format;
    if (made?.key === key) return made;
    status = { text: 'Rendering…', progress: 0 };
    const r = await output.render($state.snapshot(doc) as AnimDoc, { format, onProgress: (d, t) => (status = { text: `Rendering frame ${d} of ${t}…`, progress: d / t }) });
    if (made) drop(made.url);
    made = { blob: r.blob, url: keep(r.blob), key };
    const problems = checkPicture({ bytes: r.blob.size, width: r.width, height: r.height, frames: r.frames });
    status = problems.length ? { text: problems.join(' '), bad: true } : { text: `Made a ${format.toUpperCase()} of ${fmtBytes(r.blob.size)}.` };
    return made;
  }

  async function download() {
    try {
      busy = true;
      const f = await makeFile();
      save(f.blob, `${slug(paint.name || 'paint')}.${EXT[format]}`);
    } catch (e) {
      if (!(e instanceof Cancelled)) status = { text: (e as Error).message || 'Couldn’t make the file. Try again.', bad: true };
    } finally {
      busy = false;
    }
  }
  const slug = (s: string) => s.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'paint';
  function save(blob: Blob, name: string) {
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 1000);
  }

  // ---- saving to cyex.tv ----

  async function saveToCyex() {
    if (!backend || busy) return;
    if (issues.length) {
      status = { text: 'Fix the marked fields first.', bad: true };
      return;
    }
    busy = true;
    const steps: Record<UploadState, string> = { uploading: 'Uploading the picture…', processing: 'cyex.tv is processing the picture…', ready: 'Picture ready.' };
    try {
      const p = clonePaint($state.snapshot(paint) as StudioPaint);
      if (mode === 'animated' && source !== 'existing') {
        let file: File;
        if (source === 'build') {
          const f = await makeFile();
          const problems = checkPicture({ bytes: f.blob.size, width: doc!.width, height: doc!.height });
          if (problems.length) throw new Error(problems.join(' '));
          file = new File([f.blob], `${slug(p.name)}.${EXT[format]}`, { type: MIME[format] });
        } else file = picked!.file;
        const pic = await backend.uploadPicture(file, (s) => (status = { text: steps[s] }));
        p.image = { id: pic.id, url: pic.url };
        existingStill = pic.still;
      }
      status = { text: 'Saving…' };
      const saved = await backend.save(p, savedId);
      savedId = saved.id;
      if (mode === 'animated' && source === 'build' && doc) rememberProject(saved.id, $state.snapshot(doc) as AnimDoc);
      paint.image = saved.paint.image;
      if (mode === 'animated') {
        // the uploaded picture is now the paint's; further edits upload a new one
        if (source === 'file') source = 'existing';
      }
      status = { text: `Saved “${saved.paint.name}” to cyex.tv.` };
      onsaved?.(saved);
    } catch (e) {
      if (!(e instanceof Cancelled)) status = { text: (e as Error).message || 'Couldn’t save. Try again.', bad: true };
    } finally {
      busy = false;
    }
  }

  // ---- opening ----

  let openDialog: HTMLDialogElement;
  let list = $state<SavedPaint[] | null>(null);
  let listError = $state<string | null>(null);
  async function showOpen() {
    if (!backend) return;
    openDialog.showModal();
    list = null;
    listError = null;
    try {
      list = await backend.listPaints();
    } catch (e) {
      listError = (e as Error).message || 'Couldn’t load the paints.';
    }
  }
  async function openById(id: string) {
    if (!backend) return;
    status = { text: 'Opening…' };
    try {
      const all = await backend.listPaints();
      const found = all.find((p) => p.id === id);
      if (!found) {
        status = { text: 'That paint doesn’t exist any more.', bad: true };
        return;
      }
      await load(found);
      status = null;
    } catch (e) {
      status = { text: (e as Error).message || 'Couldn’t open the paint.', bad: true };
    }
  }
  async function load(s: SavedPaint) {
    openDialog?.close();
    savedId = s.id;
    paint = clonePaint(s.paint);
    warnings = [];
    if (s.paint.kind === 'IMAGE') {
      const E = await engine();
      const kept = recallProject(s.id);
      if (kept) {
        const d = E.Engine.sanitizeDoc(kept).doc;
        await cachePictures(E, d);
        doc = d;
        source = 'build';
      } else {
        doc = doc ?? E.Engine.sanitizeDoc(E.ANIM_PRESETS[0]()).doc;
        source = 'existing';
      }
      existingStill = s.still;
      mode = 'animated';
    } else {
      gradientKind = s.paint.kind as typeof gradientKind;
      mode = 'static';
    }
  }

  // ---- import and export ----

  let importDialog: HTMLDialogElement;
  let importText = $state('');
  let importError = $state<string | null>(null);
  async function runImport(text: string) {
    importError = null;
    let json: unknown;
    try {
      json = JSON.parse(text);
    } catch {
      importError = 'That isn’t valid JSON. Check it was copied whole.';
      return;
    }
    try {
      const pdoc = projectDoc(json);
      if (pdoc) {
        const E = await engine();
        const { doc: d, dropped } = E.Engine.sanitizeDoc(pdoc);
        await cachePictures(E, d);
        doc = d;
        paint.kind = 'IMAGE';
        paint.shadows = engineShadows(d.shadows);
        if (!paint.name) paint.name = [...d.name].slice(0, LIMITS.nameMax).join('');
        source = 'build';
        mode = 'animated';
        warnings = dropped ? [`${dropped} layers or effects this studio doesn’t have were left out.`] : [];
      } else {
        const r = importPaint(json);
        const keepId = savedId;
        paint = r.paint;
        savedId = keepId;
        warnings = r.warnings;
        if (r.paint.kind === 'IMAGE') {
          await engine();
          source = 'existing';
          existingStill = null;
          mode = 'animated';
        } else {
          gradientKind = r.paint.kind as typeof gradientKind;
          mode = 'static';
        }
      }
      importDialog.close();
      status = { text: warnings.length ? 'Imported, with changes (listed under the name).' : 'Imported.' };
    } catch (e) {
      importError = (e as Error).message || 'Couldn’t read that paint.';
    }
  }
  async function importFile(f: File | undefined) {
    if (f) await runImport(await f.text());
  }

  let exportTab = $state<'cyex' | 'int' | 'css'>('cyex');
  const exported = $derived(exportTab === 'cyex' ? exportCyex(paint) : exportTab === 'int' ? exportPaint(paint) : paintCss(paint));
  let copied = $state(false);
  async function copy() {
    try {
      await navigator.clipboard.writeText(exported);
      copied = true;
      setTimeout(() => (copied = false), 1600);
    } catch {
      status = { text: 'Couldn’t copy. Select the text and copy it yourself.', bad: true };
    }
  }
  function downloadProject() {
    if (!doc) return;
    const d = { ...($state.snapshot(doc) as AnimDoc), name: paint.name || doc.name, shadows: toEngineShadows(paint.shadows) };
    save(new Blob([JSON.stringify({ cyexPaintProject: 1, doc: d })], { type: 'application/json' }), `${slug(paint.name)}.cyexpaint.json`);
  }

  async function surpriseStatic() {
    const E = await engine();
    for (let i = 0; i < 8; i++) {
      const r = importPaint((E as unknown as { Surprise: { makeStatic(): unknown } }).Surprise.makeStatic());
      if (r.paint.stops.length <= LIMITS.stopsMax && r.warnings.length === 0) {
        Object.assign(paint, { kind: r.paint.kind, angle: r.paint.angle, repeat: r.paint.repeat, shape: r.paint.shape, stops: r.paint.stops, shadows: r.paint.shadows, color: r.paint.color });
        gradientKind = r.paint.kind as typeof gradientKind;
        return;
      }
    }
  }

  function onkey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's' && backend) {
      e.preventDefault();
      void saveToCyex();
    }
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="ps">
  <div class="ps-top">
    <div class="ps-field ps-name">
      <label for="ps-name">Paint name <span class="ps-count" class:ps-count-bad={nameLength > LIMITS.nameMax}>{nameLength} of {LIMITS.nameMax}</span></label>
      <input id="ps-name" class="ps-text" type="text" autocomplete="off" spellcheck="false" placeholder="Sunset Strip" bind:value={paint.name} aria-invalid={nameIssue && paint.name ? 'true' : undefined} aria-describedby="ps-name-m" />
      <p id="ps-name-m" class={nameIssue && paint.name ? 'ps-msg' : 'ps-hint'}>{nameIssue && paint.name ? nameIssue : savedId ? 'Editing a saved paint. Saving replaces it.' : 'A new paint. Names are unique on cyex.tv.'}</p>
    </div>
    <div class="ui-actions">
      <button type="button" class="ui-btn secondary" onclick={() => (importText = '', importError = null, importDialog.showModal())}><Glyph name="upload" /> Import</button>
      {#if backend}
        <button type="button" class="ui-btn secondary" onclick={showOpen}><Glyph name="folder" /> Open</button>
        <button type="button" class="ui-btn primary" disabled={busy} onclick={saveToCyex} aria-describedby="ps-status">{savedId ? 'Save changes' : 'Save to cyex.tv'}</button>
      {/if}
    </div>
  </div>

  <div class="ps-status" class:bad={status?.bad} id="ps-status" role="status" aria-live="polite">
    {#if status}
      <span>{status.text}</span>
      {#if status.progress != null}<span class="ps-progress" aria-hidden="true"><span style:width="{Math.round(status.progress * 100)}%"></span></span>{/if}
    {/if}
  </div>
  {#if warnings.length}
    <div class="ps-panel" role="note" style:margin-bottom="16px">
      <div class="ps-body">
        <strong>The import changed a few things</strong>
        <ul class="ps-issues" style:color="var(--muted)">{#each warnings as w (w)}<li>{w}</li>{/each}</ul>
        <div><button type="button" class="ui-btn ghost small" onclick={() => (warnings = [])}>Dismiss</button></div>
      </div>
    </div>
  {/if}

  <div style:margin-bottom="16px">
    <Seg big label="Paint type" value={mode} onchange={setMode} options={[{ value: 'static', label: 'Gradient', icon: 'palette' }, { value: 'animated', label: 'Animated or picture', icon: 'image' }]} />
  </div>

  <div class="ps-mini" aria-hidden="true">
    <span><span class="ps-painted ps-mid" style={paintStyleText(paint, previewImage)}>{previewName.trim() || 'lumen_fox'}</span></span>
    <span><span class="ps-painted ps-mid" style={paintStyleText(paint, previewImage)}>{previewName.trim() || 'lumen_fox'}</span></span>
  </div>

  <div class="ps-layout">
    <div class="ps-col">
      {#if mode === 'static'}
        <StaticEditor {paint} {issues} onchange={(patch) => Object.assign(paint, patch)} onsurprise={surpriseStatic} />
      {:else}
        <section class="ps-panel" aria-labelledby="ps-src-h">
          <header><h2 id="ps-src-h">Picture</h2></header>
          <div class="ps-body">
            <Seg
              label="Where the picture comes from"
              value={source}
              onchange={(v) => (source = v)}
              options={[
                { value: 'build', label: 'Build it here', icon: 'brush' },
                { value: 'file', label: 'Upload a file', icon: 'upload' },
                ...(paint.image?.url ? [{ value: 'existing' as const, label: 'Keep the saved one' }] : []),
              ]}
            />
            {#if source === 'file'}
              <div class="ps-field">
                <label for="ps-file">GIF, PNG, APNG or WebP</label>
                <input id="ps-file" type="file" accept="image/gif,image/png,image/apng,image/webp" onchange={(e) => pickFile(e.currentTarget.files?.[0])} />
                {#if picked}<p class="ps-hint">{picked.width} × {picked.height} px, {fmtBytes(picked.file.size)}</p>{/if}
                {#each pickError as m (m)}<p class="ps-msg" role="alert">{m}</p>{/each}
                <p class="ps-hint">At least 64 px tall, at most 1.000 px a side and 5 MB. Up to 8:1; wider pictures are scaled to fit.</p>
              </div>
            {:else if source === 'existing'}
              {#if paint.image?.url}<div class="ps-imgbox"><img src={previewImage ?? paint.image.url} alt="Saved for this paint" /></div>{/if}
              <p class="ps-hint">Saving keeps this picture and updates the name, shadows and fallback colour.</p>
            {:else}
              <p class="ps-hint">The layers below are rendered into a looping picture when you save or download it.</p>
            {/if}
          </div>
        </section>
        {#if source === 'build' && PL && doc}
          <AnimEditor {PL} bind:doc onreplace={(d) => (doc = d)} />
        {/if}
      {/if}

      <section class="ps-panel" aria-labelledby="ps-look-h">
        <header><h2 id="ps-look-h">Shadows and fallback</h2></header>
        <div class="ps-body">
          <Shadows shadows={paint.shadows} issues={issues.filter((i) => i.field === 'shadows')} onchange={(s) => (paint.shadows = s)} />
          <label class="ui-check"><input class="ui-switch" type="checkbox" role="switch" checked={paint.color != null} onchange={(e) => (paint.color = e.currentTarget.checked ? (paint.stops[0]?.color.slice(0, 7) ?? '#FFFFFF') : null)} /> Fallback colour</label>
          {#if paint.color != null}
            <Colour label="Fallback colour" value={paint.color} error={issues.find((i) => i.field === 'color')?.message} onchange={(c) => (paint.color = c)} />
          {/if}
          <p class="ps-hint">Shown where a chat can’t draw paints.</p>
          {#if mode === 'animated'}
            <label class="ui-check"><input class="ui-switch" type="checkbox" role="switch" checked={paint.repeat} onchange={(e) => (paint.repeat = e.currentTarget.checked)} /> Tile the picture along long names</label>
          {/if}
        </div>
      </section>
    </div>

    <div class="ps-side">
      {#if mode === 'animated'}
        <div class="ps-row" style:justify-content="space-between">
          <span class="ps-status" role="status">
            {#if renderError && source === 'build'}<span class="ps-msg">{renderError}</span>
            {:else if renderProgress != null && source === 'build'}Rendering preview {Math.round(renderProgress * 100)}%{:else if reduced && !playing}Paused for reduced motion{/if}
          </span>
          <button type="button" class="ui-btn secondary small" aria-pressed={playing} onclick={() => (playing = !playing)}><Glyph name={playing ? 'pause' : 'play'} /> {playing ? 'Pause' : 'Play'}</button>
        </div>
      {/if}
      <Preview {paint} imageUrl={previewImage} name={previewName} onname={(n) => (previewName = n)} />

      {#if issues.length}
        <section class="ps-panel" aria-labelledby="ps-issues-h">
          <header><h2 id="ps-issues-h">Before it can be saved</h2><span class="ps-count">{issues.length}</span></header>
          <div class="ps-body"><ul class="ps-issues">{#each issues as i, k (k)}<li>{i.message}</li>{/each}</ul></div>
        </section>
      {/if}

      <section class="ps-panel" aria-labelledby="ps-export-h">
        <header><h2 id="ps-export-h">Export</h2></header>
        <div class="ps-body">
          {#if mode === 'animated' && source === 'build'}
            <div class="ps-field">
              <span class="ps-flabel">File type</span>
              <Seg label="File type" value={format} onchange={(v) => (format = v)} options={[{ value: 'webp', label: 'WebP' }, { value: 'apng', label: 'APNG' }, { value: 'gif', label: 'GIF' }]} />
              <p class="ps-hint">WebP is usually the smallest. GIF has 256 colours and hard edges. This is also the file saving uploads.</p>
            </div>
            <div class="ui-actions">
              <button type="button" class="ui-btn secondary small" disabled={busy} onclick={download}><Glyph name="download" /> Download picture</button>
              <button type="button" class="ui-btn ghost small" onclick={downloadProject}>Download project</button>
            </div>
          {/if}
          <Seg label="Export as" value={exportTab} onchange={(v) => (exportTab = v)} options={[{ value: 'cyex', label: 'cyex.tv JSON' }, { value: 'int', label: 'Integer JSON' }, { value: 'css', label: 'CSS' }]} />
          <pre class="ps-code" aria-label="Exported paint">{exported}</pre>
          <div class="ui-actions"><button type="button" class="ui-btn secondary small" onclick={copy}><Glyph name={copied ? 'check' : 'copy'} /> {copied ? 'Copied' : 'Copy'}</button></div>
          <p class="ps-hint">Integer JSON is the single-gradient format other paint tools read, with colours as packed integers.</p>
        </div>
      </section>
    </div>
  </div>
</div>

<dialog class="ui-dialog ps-dialog" bind:this={importDialog} aria-labelledby="ps-import-h">
  <header>
    <h2 id="ps-import-h">Import a paint</h2>
    <button type="button" class="ui-icon" aria-label="Close" title="Close" onclick={() => importDialog.close()}><Glyph name="x" /></button>
  </header>
  <div class="scroll">
    <p class="ps-hint">Paste paint JSON: a cyex.tv paint, single-gradient JSON from other paint tools (integer or hex colours), or a cyex paints project file.</p>
    <div class="ps-field">
      <label for="ps-import-t">JSON</label>
      <textarea id="ps-import-t" class="ps-text" spellcheck="false" bind:value={importText} aria-invalid={importError ? 'true' : undefined} aria-describedby="ps-import-e"></textarea>
      {#if importError}<p class="ps-msg" id="ps-import-e" role="alert">{importError}</p>{/if}
    </div>
    <div class="ps-field">
      <label for="ps-import-f">Or open a file</label>
      <input id="ps-import-f" type="file" accept="application/json,.json" onchange={(e) => importFile(e.currentTarget.files?.[0])} />
    </div>
  </div>
  <footer>
    <button type="button" class="ui-btn secondary" onclick={() => importDialog.close()}>Cancel</button>
    <button type="button" class="ui-btn primary" disabled={!importText.trim()} onclick={() => runImport(importText)}>Import</button>
  </footer>
</dialog>

<dialog class="ui-dialog ps-dialog" bind:this={openDialog} aria-labelledby="ps-open-h">
  <header>
    <h2 id="ps-open-h">Open a cyex.tv paint</h2>
    <button type="button" class="ui-icon" aria-label="Close" title="Close" onclick={() => openDialog.close()}><Glyph name="x" /></button>
  </header>
  <div class="scroll">
    {#if listError}<p class="ps-msg" role="alert">{listError}</p>
    {:else if !list}<p class="ps-hint" role="status">Loading paints…</p>
    {:else if !list.length}<p class="ps-hint">No paints yet.</p>
    {:else}
      <div class="ps-cards">
        {#each list as s (s.id)}
          <button type="button" class="ps-card" onclick={() => load(s)}>
            <span class="ps-stage dark ps-chip"><b class="ps-painted ps-mid" style={paintStyleText(s.paint, s.paint.kind === 'IMAGE' ? (reduced ? s.still : s.paint.image?.url) : null)}>{s.paint.name}</b></span>
            <span>{s.paint.kind === 'IMAGE' ? 'Picture' : s.paint.kind === 'LINEAR_GRADIENT' ? 'Linear gradient' : 'Radial gradient'}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
  <footer><button type="button" class="ui-btn secondary" onclick={() => openDialog.close()}>Cancel</button></footer>
</dialog>
