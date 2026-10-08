import rss from '@astrojs/rss';
import { getCollection } from 'astro:content';
import { sortByDate } from '../lib/content';

export async function GET(context: { site?: URL }) {
  const updates = sortByDate(await getCollection('updates'));
  const site = context.site ?? new URL('http://localhost:4321');
  const base = import.meta.env.BASE_URL === '/' ? '' : import.meta.env.BASE_URL.replace(/\/$/, '');
  return rss({
    title: 'Ryujin Hatakeyama — Updates',
    description: 'Research, academic, and writing updates.',
    site,
    items: updates.map((entry) => ({
      title: entry.data.title.en,
      description: entry.data.summary.en,
      pubDate: entry.data.announcedOn,
      link: entry.data.detail ? `${base}/updates/item/${entry.data.id}/` : `${base}/updates/`,
      categories: entry.data.categories
    })),
    customData: '<language>en</language>'
  });
}
