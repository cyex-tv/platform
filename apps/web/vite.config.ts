import { sveltekit } from '@sveltejs/kit/vite';
import { svelteTesting } from '@testing-library/svelte/vite';
import { defineConfig } from 'vitest/config';

// Two projects: `unit` (plain logic, node) and `ui` (components in jsdom, *.ui.test.ts).
export default defineConfig({
  plugins: [sveltekit()],
  test: {
    coverage: { provider: 'v8', include: ['src/**/*.{ts,svelte}'], exclude: ['src/lib/gql/**', '**/*.test.ts', '**/*.d.ts'], reporter: ['text-summary'] },
    projects: [
      {
        extends: true,
        test: {
          name: 'unit',
          // The paint studio's render tests take about 3 s alone and more when every worker is busy.
          testTimeout: 20_000,
          include: ['src/**/*.test.ts', 'dev/**/*.test.mjs'],
          exclude: ['src/**/*.ui.test.ts'],
        },
      },
      {
        extends: true,
        plugins: [svelteTesting()],
        test: {
          name: 'ui',
          environment: 'jsdom',
          include: ['src/**/*.ui.test.ts'],
          setupFiles: ['./src/lib/test/dom-setup.ts'],
        },
      },
    ],
  },
});
