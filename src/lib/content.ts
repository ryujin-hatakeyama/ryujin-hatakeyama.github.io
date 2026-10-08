export type Locale = 'en' | 'ja';

export const categoryLabels = {
  en: { research: 'Research', academia: 'Academia', writing: 'Writing' },
  ja: { research: '研究', academia: 'アカデミア', writing: '執筆' }
} as const;

export const kindLabels = {
  en: {
    acceptance: 'Acceptance', presentation: 'Presentation', publication: 'Publication',
    participation: 'Participation', visit: 'Visit', award: 'Award', release: 'Release', other: 'Other'
  },
  ja: {
    acceptance: '採択', presentation: '発表', publication: '公開', participation: '参加',
    visit: '訪問', award: '受賞・選考', release: 'リリース', other: 'その他'
  }
} as const;

export function formatDate(date: Date | string, locale: Locale): string {
  const value = typeof date === 'string' ? new Date(`${date}T00:00:00Z`) : date;
  return new Intl.DateTimeFormat(locale === 'ja' ? 'ja-JP' : 'en-GB', {
    year: 'numeric', month: locale === 'ja' ? 'numeric' : 'short', day: '2-digit', timeZone: 'UTC'
  }).format(value);
}

export function sortByDate<T extends { data: { date?: Date; announcedOn?: Date } }>(items: T[]): T[] {
  return [...items].sort((a, b) => {
    const aDate = a.data.announcedOn ?? a.data.date;
    const bDate = b.data.announcedOn ?? b.data.date;
    return (bDate?.getTime() ?? 0) - (aDate?.getTime() ?? 0);
  });
}

