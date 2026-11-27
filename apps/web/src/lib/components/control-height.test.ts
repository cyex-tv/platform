// Filter rows read one control height, --ctl-h (tool.css): fields, selects, buttons, the date button and the
// person picker are all it, so a row that mixes them is level (the audit log's "Any actor" was 34px beside 40px
// neighbours). jsdom has no layout, so the rules are checked here and the heights are measured in the running app.
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const src = (p: string) => readFileSync(new URL(p, import.meta.url), 'utf8');

describe('one control height', () => {
  it('tool.css defines it, with the compact and touch sizes, and fields and buttons read it', () => {
    const css = src('../../tool.css');
    expect(css).toMatch(/:root\s*\{\s*--ctl-h:\s*40px/);
    expect(css).toMatch(/\.adm-toolbar,\s*\.ui-compact\s*\{\s*--ctl-h:\s*34px/);
    expect(css).toMatch(/\.ui-btn\s*\{[^}]*height:\s*var\(--ctl-h\)/);
    expect(css).toMatch(/input\.ui-input, select\.ui-input\s*\{\s*height:\s*var\(--ctl-h\)/);
    expect(css).toMatch(/@media \(pointer: coarse\) \{\s*:root, \.adm-toolbar, \.ui-compact \{ --ctl-h: 44px/);
  });
  it('the person picker, the date button and the range button follow it', () => {
    const uf = src('./UserFilter.svelte');
    expect(uf).not.toMatch(/height:\s*34px/);
    expect(uf.match(/height:\s*var\(--ctl-h, 34px\)/g)?.length).toBe(3);
    expect(src('./DateField.svelte')).toMatch(/\.date\s*\{\s*height:\s*var\(--ctl-h, 40px\)/);
    expect(src('../profile/DateRangePicker.svelte')).toMatch(/\.ui-btn\.trig\s*\{[^}]*height:\s*var\(--ctl-h, 34px\)/);
  });
  it('the audit log row no longer sets heights of its own', () => {
    expect(src('../../routes/(tool)/admin/audit/+page.svelte')).not.toMatch(/\.filters[^{]*\{[^}]*height:/);
  });
});
