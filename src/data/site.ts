export const siteInfo = {
  name: 'Ryujin Hatakeyama',
  nameJapanese: '畠山竜迅',
  affiliation: null as string | null,
  affiliationJapanese: null as string | null,
  pronunciation: {
    reading: 'はたけやま りゅうじん',
    ipa: 'hatakejama ɾʲɯːdʑiɴ'
  } as null | {
    reading: string;
    romanization?: string;
    ipa?: string;
    audio?: string;
  },
  cv: {
    href: '/cv/',
    label: 'CV',
    labelJapanese: 'CV（英語）'
  } as null | { href: string; label?: string; labelJapanese?: string },
  contacts: [
    { label: 'GitHub', href: 'https://github.com/ryujin-hatakeyama' }
  ] as Array<{ label: string; href: string }>,
  academicLinks: [] as Array<{ label: string; href: string }>
} as const;
