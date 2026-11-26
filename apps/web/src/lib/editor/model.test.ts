import { describe, expect, it } from 'vitest';
import {
  addShape,
  boomerangFrames,
  canvasSize,
  createDoc,
  cropNoWider,
  deleteFrames,
  duplicateFrames,
  fitTo,
  flipH,
  outSize,
  resize,
  resizeKeep,
  rotateCw,
  setCrop,
  shapeBounds,
  shapesOn,
  trim,
  type Shape,
} from './model';
import { History } from './history';

const line = (pts: number[], frame: number | null = null): Omit<Shape, 'id'> => ({
  kind: 'line',
  pts,
  color: '#fff',
  fill: '#000',
  width: 2,
  filled: false,
  outlined: true,
  frame,
});

describe('document', () => {
  it('starts as the source, one frame per picture', () => {
    const d = createDoc(200, 100, [40, 40, 40]);
    expect(outSize(d)).toEqual({ w: 200, h: 100 });
    expect(d.frames.map((f) => f.src)).toEqual([0, 1, 2]);
  });

  it('turns the crop, the output and the drawings with the picture', () => {
    let d = createDoc(200, 100, [0]);
    d = setCrop(d, { x: 150, y: 0, w: 50, h: 20 });
    d = resize(d, 100, 40);
    d = addShape(d, line([190, 10, 199, 10])).doc;
    const r = rotateCw(d);
    expect(canvasSize(r)).toEqual({ w: 100, h: 200 });
    expect(r.crop).toEqual({ x: 80, y: 150, w: 20, h: 50 });
    expect(outSize(r)).toEqual({ w: 40, h: 100 });
    expect(r.shapes[0].pts).toEqual([90, 190, 90, 199]);
    // four turns: back where it started
    expect(rotateCw(rotateCw(rotateCw(r)))).toMatchObject({ crop: d.crop, sx: d.sx, sy: d.sy });
  });

  it('mirrors drawings', () => {
    const d = addShape(createDoc(100, 50, [0]), line([10, 5, 20, 5])).doc;
    expect(flipH(d).shapes[0].pts).toEqual([90, 5, 80, 5]);
  });

  it('resizes keeping the shape, stretches, and fits', () => {
    const d = createDoc(400, 100, [0]);
    expect(outSize(resizeKeep(d, 200))).toEqual({ w: 200, h: 50 });
    expect(outSize(resize(d, 100, 100))).toEqual({ w: 100, h: 100 });
    expect(outSize(fitTo(createDoc(3000, 1999, [0]), 1000))).toEqual({ w: 1000, h: 666 });
    expect(outSize(fitTo(createDoc(999, 3001, [0]), 1000)).h).toBe(1000);
    // a stretched output stays stretched in proportion
    expect(outSize(resizeKeep(resize(d, 400, 200), 200))).toEqual({ w: 200, h: 100 });
  });

  it('crops to no wider than 4:1, in the middle', () => {
    const d = cropNoWider(createDoc(1000, 100, [0]), 4);
    expect(d.crop).toEqual({ x: 300, y: 0, w: 400, h: 100 });
    expect(cropNoWider(createDoc(300, 100, [0]), 4).crop.w).toBe(300);
  });

  it('deletes, duplicates and trims frames, with their own drawings', () => {
    let d = createDoc(10, 10, [10, 20, 30, 40]);
    d = addShape(d, line([0, 0, 1, 1], d.frames[1].id)).doc;
    d = addShape(d, line([0, 0, 1, 1])).doc;
    const dup = duplicateFrames(d, [1]);
    expect(dup.frames.map((f) => f.delay)).toEqual([10, 20, 20, 30, 40]);
    expect(shapesOn(dup, dup.frames[2].id)).toHaveLength(2);
    const del = deleteFrames(d, [1]);
    expect(del.frames).toHaveLength(3);
    expect(del.shapes).toHaveLength(1);
    expect(deleteFrames(createDoc(1, 1, [0]), [0]).frames).toHaveLength(1);
    expect(trim(d, 1, 2).frames.map((f) => f.delay)).toEqual([20, 30]);
  });

  it('boomerangs with new frame ids', () => {
    const d = boomerangFrames(createDoc(1, 1, [1, 2, 3]));
    expect(d.frames.map((f) => f.delay)).toEqual([1, 2, 3, 2]);
    expect(new Set(d.frames.map((f) => f.id)).size).toBe(4);
  });

  it('measures a drawing', () => {
    expect(shapeBounds({ ...line([5, 9, 1, 3]), id: 1 })).toEqual({ x: 1, y: 3, w: 4, h: 6 });
  });
});

describe('history', () => {
  it('undoes and redoes every step, and a new step clears redo', () => {
    const h = new History(0);
    h.push(1, 'One');
    h.push(2, 'Two');
    expect(h.undoLabel).toBe('Two');
    expect(h.undo()).toBe(1);
    expect(h.redoLabel).toBe('Two');
    expect(h.redo()).toBe(2);
    h.undo();
    h.push(3, 'Three');
    expect(h.canRedo).toBe(false);
    expect(h.undo()).toBe(1);
    expect(h.undo()).toBe(0);
    expect(h.canUndo).toBe(false);
    expect(h.undo()).toBe(0);
  });

  it('skips a step that changes nothing and keeps at most `limit`', () => {
    const h = new History(0, 3);
    h.push(0, 'Nothing');
    expect(h.depth).toBe(0);
    for (let i = 1; i <= 5; i++) h.push(i, `S${i}`);
    expect(h.depth).toBe(3);
    h.undo();
    h.undo();
    h.undo();
    expect(h.now).toBe(2);
  });
});
