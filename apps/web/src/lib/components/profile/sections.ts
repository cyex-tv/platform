// A profile page's sections are query-param tabs (?s=chat): shareable, and back/forward work.
// The first section is the default and has no parameter.
import type { RailGroup, RailItem } from '$lib/shell/rail';

export type Section<K extends string> = Omit<RailItem, 'href' | 'current'> & { key: K };

/** The section the address asks for; anything unknown is the first. */
export function currentSection<K extends string>(sections: Section<K>[], search: URLSearchParams): K {
  const want = search.get('s');
  return sections.find((s) => s.key === want)?.key ?? sections[0].key;
}

/** The rail's rows for the sections, with `current` set. */
export function sectionRail<K extends string>(sections: Section<K>[], path: string, current: K, caption?: string): RailGroup[] {
  return [
    {
      caption,
      items: sections.map(({ key, ...rest }, i) => ({
        ...rest,
        href: i === 0 ? path : `${path}?s=${key}`,
        current: key === current,
      })),
    },
  ];
}
