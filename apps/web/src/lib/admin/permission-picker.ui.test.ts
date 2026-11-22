import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import PermissionPicker from './PermissionPicker.svelte';

const available = ['emotes.upload', 'emotes.review', 'users.block', 'users.restrict'];
const radio = (perm: string, name: string) => within(screen.getByRole('radiogroup', { name: perm })).getByRole('radio', { name });
const props = (o: Record<string, unknown> = {}) => ({ available, allows: [] as string[], denies: [] as string[], canChange: () => true, ...o });

describe('the role editor’s three-way permission control', () => {
  it('is a radio group per permission, Inherit by default, and Deny marks it checked', async () => {
    const user = userEvent.setup();
    render(PermissionPicker, { props: props() });
    expect(radio('Upload emotes', 'Inherit').getAttribute('aria-checked')).toBe('true');
    await user.click(radio('Upload emotes', 'Deny'));
    expect(radio('Upload emotes', 'Deny').getAttribute('aria-checked')).toBe('true');
    expect(radio('Upload emotes', 'Inherit').getAttribute('aria-checked')).toBe('false');
    await user.click(radio('Upload emotes', 'Allow'));
    expect(radio('Upload emotes', 'Allow').getAttribute('aria-checked')).toBe('true');
  });

  it('moves with the arrow keys, one tab stop per row', async () => {
    const user = userEvent.setup();
    render(PermissionPicker, { props: props() });
    radio('Upload emotes', 'Inherit').focus();
    expect(radio('Upload emotes', 'Allow').tabIndex).toBe(-1);
    await user.keyboard('{ArrowRight}');
    expect(radio('Upload emotes', 'Deny').getAttribute('aria-checked')).toBe('true');
    expect(document.activeElement).toBe(radio('Upload emotes', 'Deny'));
    await user.keyboard('{ArrowRight}');
    expect(radio('Upload emotes', 'Allow').getAttribute('aria-checked')).toBe('true');
    await user.keyboard('{ArrowLeft}');
    expect(radio('Upload emotes', 'Deny').getAttribute('aria-checked')).toBe('true');
  });

  it('sets a whole group with its buttons, skipping what you may not change', async () => {
    const user = userEvent.setup();
    render(PermissionPicker, { props: props({ canChange: (k: string) => k !== 'users.restrict' }) });
    await user.click(screen.getByRole('button', { name: 'Deny all in Moderation' }));
    expect(radio('Block accounts', 'Deny').getAttribute('aria-checked')).toBe('true');
    expect(radio('All restrictions', 'Inherit').getAttribute('aria-checked')).toBe('true');
    expect(radio('Upload emotes', 'Inherit').getAttribute('aria-checked')).toBe('true');
    await user.click(screen.getByRole('button', { name: 'Allow all in Emotes' }));
    expect(radio('Review uploads', 'Allow').getAttribute('aria-checked')).toBe('true');
    await user.click(screen.getByRole('button', { name: 'Inherit all in Emotes' }));
    expect(radio('Review uploads', 'Inherit').getAttribute('aria-checked')).toBe('true');
  });

  it('disables what you do not hold, and offers no Deny on the User role', () => {
    const { unmount } = render(PermissionPicker, { props: props({ canChange: (k: string) => k !== 'users.block' }) });
    expect((radio('Block accounts', 'Allow') as HTMLButtonElement).disabled).toBe(true);
    expect((radio('Review uploads', 'Allow') as HTMLButtonElement).disabled).toBe(false);
    unmount();
    render(PermissionPicker, { props: props({ noDeny: true }) });
    expect(within(screen.getByRole('radiogroup', { name: 'Upload emotes' })).queryByRole('radio', { name: 'Deny' })).toBeNull();
    expect(screen.queryByRole('button', { name: /^Deny all/ })).toBeNull();
  });
});
