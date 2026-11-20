import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import TagInput from './TagInput.svelte';

const field = () => screen.getByRole('textbox', { name: 'Tags' }) as HTMLInputElement;
const setup = (tags: string[] = [], extra: Record<string, unknown> = {}) => {
  const onchange = vi.fn();
  render(TagInput, { tags, onchange, label: 'Tags', ...extra });
  return { onchange, user: userEvent.setup() };
};

describe('TagInput in the browser', () => {
  it('Enter turns the text into a tag and empties the field', async () => {
    const { onchange, user } = setup(['cat']);
    await user.type(field(), 'Dog{Enter}');
    expect(onchange).toHaveBeenCalledWith(['cat', 'dog']);
    expect(field().value).toBe('');
  });

  it('a comma ends a tag too', async () => {
    const { onchange, user } = setup();
    await user.type(field(), 'a,');
    expect(onchange).toHaveBeenLastCalledWith(['a']);
    expect(field().value).toBe('');
  });

  it('Enter on an empty field adds nothing (a form can submit)', async () => {
    const { onchange, user } = setup();
    await user.click(field());
    await user.keyboard('{Enter}');
    expect(onchange).not.toHaveBeenCalled();
  });

  it('Backspace in the empty field takes the last tag off; with text it only deletes a character', async () => {
    const { onchange, user } = setup(['one', 'two']);
    await user.click(field());
    await user.keyboard('x{Backspace}');
    expect(onchange).not.toHaveBeenCalled();
    await user.keyboard('{Backspace}');
    expect(onchange).toHaveBeenCalledWith(['one']);
  });

  it('pasting a list adds every tag at once, without repeats', async () => {
    const { onchange, user } = setup(['a']);
    await user.click(field());
    await user.paste('a, b, c\nd');
    expect(onchange).toHaveBeenCalledTimes(1);
    expect(onchange).toHaveBeenCalledWith(['a', 'b', 'c', 'd']);
  });

  it('a paste without a separator is typed text, not a tag yet', async () => {
    const { onchange, user } = setup();
    await user.click(field());
    await user.paste('single');
    expect(onchange).not.toHaveBeenCalled();
    expect(field().value).toBe('single');
  });

  it('stops at the maximum and says so', async () => {
    const { onchange, user } = setup(['a', 'b'], { max: 2 });
    await user.type(field(), 'c{Enter}');
    expect(onchange).not.toHaveBeenCalled();
    expect(screen.getAllByText('Up to 2 tags.').length).toBeGreaterThan(0);
  });

  it('each chip has a remove button', async () => {
    const { onchange, user } = setup(['one', 'two']);
    await user.click(screen.getByRole('button', { name: 'Remove #one' }));
    expect(onchange).toHaveBeenCalledWith(['two']);
  });
});
