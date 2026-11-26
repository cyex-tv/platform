// Drawing an editor frame: the source picture turned into canvas space, then the drawings on it.
// The same code paints the stage (canvas space under the view's zoom and pan), the timeline's
// thumbnails and the output frames the encoder gets (canvas space cropped and scaled), so what
// you see is what uploads.
import { chain, type Affine } from './geometry';
import { orientTransform, outSize, outputTransform, shapesOn, type Doc, type Shape } from './model';

type Ctx = CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D;

/** The text font: the site's own, bold. */
export const TEXT_FONT = (size: number) => `700 ${Math.max(1, Math.round(size))}px "IBM Plex Sans", "Geist", system-ui, sans-serif`;

/**
 * Paints frame `index` of `d` with `view` mapping canvas space to the context's pixels.
 * `src` is that frame's source picture.
 */
export function paintFrame(ctx: Ctx, d: Doc, index: number, src: CanvasImageSource, view: Affine, opts: { shapes?: boolean; alpha?: number } = {}) {
  const frame = d.frames[index];
  if (!frame) return;
  ctx.save();
  ctx.globalAlpha = opts.alpha ?? 1;
  ctx.setTransform(...chain(view, orientTransform(d)));
  ctx.drawImage(src, 0, 0, d.sw, d.sh);
  ctx.restore();
  if (opts.shapes !== false) {
    ctx.save();
    ctx.globalAlpha = opts.alpha ?? 1;
    ctx.setTransform(...view);
    for (const s of shapesOn(d, frame.id)) drawShape(ctx, s);
    ctx.restore();
  }
}

/** Paints output frame `index`: cropped, scaled, with its drawings, on a cleared context. */
export function paintOutput(ctx: Ctx, d: Doc, index: number, src: CanvasImageSource) {
  const o = outSize(d);
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, o.w, o.h);
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = 'high';
  paintFrame(ctx, d, index, src, outputTransform(d));
}

export function drawShape(ctx: Ctx, s: Shape) {
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  ctx.strokeStyle = s.color;
  ctx.lineWidth = s.width;
  const p = s.pts;
  switch (s.kind) {
    case 'pen': {
      ctx.beginPath();
      if (p.length <= 2) {
        // A dot: one tap with the pen.
        ctx.fillStyle = s.color;
        ctx.arc(p[0], p[1], s.width / 2, 0, Math.PI * 2);
        ctx.fill();
        return;
      }
      ctx.moveTo(p[0], p[1]);
      // Through the midpoints, so the line is smooth rather than a polygon.
      for (let i = 2; i + 3 < p.length; i += 2) ctx.quadraticCurveTo(p[i], p[i + 1], (p[i] + p[i + 2]) / 2, (p[i + 1] + p[i + 3]) / 2);
      ctx.lineTo(p[p.length - 2], p[p.length - 1]);
      ctx.stroke();
      return;
    }
    case 'line':
    case 'arrow': {
      ctx.beginPath();
      ctx.moveTo(p[0], p[1]);
      ctx.lineTo(p[2], p[3]);
      if (s.kind === 'arrow') {
        const a = Math.atan2(p[3] - p[1], p[2] - p[0]);
        const len = Math.max(8, s.width * 4);
        for (const k of [-1, 1]) {
          ctx.moveTo(p[2], p[3]);
          ctx.lineTo(p[2] - len * Math.cos(a + k * 0.5), p[3] - len * Math.sin(a + k * 0.5));
        }
      }
      ctx.stroke();
      return;
    }
    case 'rect':
    case 'ellipse': {
      const [x, y, w, h] = p;
      ctx.beginPath();
      if (s.kind === 'rect') ctx.rect(x, y, w, h);
      else ctx.ellipse(x + w / 2, y + h / 2, Math.abs(w) / 2, Math.abs(h) / 2, 0, 0, Math.PI * 2);
      if (s.filled) {
        ctx.fillStyle = s.fill;
        ctx.fill();
      }
      if (s.outlined || !s.filled) ctx.stroke();
      return;
    }
    case 'text': {
      const [x, y] = p;
      ctx.font = TEXT_FONT(s.size ?? 32);
      ctx.textBaseline = 'top';
      const lines = (s.text ?? '').split('\n');
      const lh = (s.size ?? 32) * 1.2;
      if (s.outlined) {
        ctx.strokeStyle = s.fill;
        ctx.lineWidth = s.width * 2;
        lines.forEach((l, i) => ctx.strokeText(l, x, y + i * lh));
      }
      ctx.fillStyle = s.color;
      lines.forEach((l, i) => ctx.fillText(l, x, y + i * lh));
      return;
    }
  }
}

/** The box of a text drawing: its widest line by its lines. */
export function measureText(ctx: Ctx, text: string, size: number): { w: number; h: number } {
  ctx.save();
  ctx.font = TEXT_FONT(size);
  const lines = text.split('\n');
  const w = Math.max(1, ...lines.map((l) => ctx.measureText(l).width));
  ctx.restore();
  return { w: Math.ceil(w), h: Math.ceil(lines.length * size * 1.2) };
}
