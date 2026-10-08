import { defineCollection } from 'astro:content';
import { file, glob } from 'astro/loaders';
import { z } from 'astro/zod';

const link = z.object({
  label: z.string().min(1),
  url: z.url(),
  type: z.enum(['external', 'pdf', 'doi', 'preprint', 'code', 'slides', 'bibtex', 'audio']).default('external')
});

const eventKind = z.discriminatedUnion('type', [
  z.object({
    type: z.literal('award'),
    outcome: z.enum(['nominated', 'shortlisted', 'won']),
    name: z.string().min(1)
  }),
  z.object({
    type: z.enum(['acceptance', 'presentation', 'publication', 'participation', 'visit', 'release', 'other']),
    detail: z.string().optional()
  })
]);

const updates = defineCollection({
  loader: file('src/data/generated/updates.json'),
  schema: z.object({
    id: z.string(),
    title: z.object({ en: z.string(), ja: z.string().optional() }),
    summary: z.object({ en: z.string(), ja: z.string().optional() }),
    date: z.coerce.date(),
    announcedOn: z.coerce.date(),
    categories: z.array(z.enum(['research', 'academia', 'writing'])).min(1),
    kind: eventKind,
    links: z.array(link).default([]),
    related: z.object({ publications: z.array(z.string()), projects: z.array(z.string()), writings: z.array(z.string()) }),
    detail: z.boolean().default(false),
    body: z.object({ en: z.string().optional(), ja: z.string().optional() }).optional()
  })
});

const publications = defineCollection({
  loader: glob({ pattern: '**/*.json', base: './src/data/publications' }),
  schema: z.object({
    slug: z.string().regex(/^[a-z0-9]+(?:-[a-z0-9]+)*$/),
    title: z.string(),
    authors: z.array(z.string()).min(1),
    year: z.number().int(),
    type: z.enum(['conference-paper', 'journal-article', 'preprint', 'extended-abstract', 'workshop-contribution', 'student-research-competition', 'presentation', 'poster', 'software']),
    status: z.enum(['draft', 'submitted', 'accepted', 'forthcoming', 'published', 'presented']),
    venue: z.string().optional(),
    note: z.string().optional(),
    presentation: z.object({
      format: z.enum(['oral', 'poster']),
      category: z.string().min(1).optional(),
      proceedingsReview: z.enum(['refereed', 'unrefereed']).optional()
    }).optional(),
    links: z.array(link).default([]),
    projectIds: z.array(z.string()).default([]),
    draft: z.boolean().default(false)
  }).superRefine((record, context) => {
    const isPresentation = record.type === 'presentation' || record.type === 'poster';
    if (isPresentation && !record.presentation) {
      context.addIssue({ code: 'custom', path: ['presentation'], message: 'presentation metadata is required for presentation and poster records' });
    }
    if (!isPresentation && record.presentation) {
      context.addIssue({ code: 'custom', path: ['presentation'], message: 'presentation metadata is only valid for presentation and poster records' });
    }
    if (record.type === 'poster' && record.presentation?.format !== 'poster') {
      context.addIssue({ code: 'custom', path: ['presentation', 'format'], message: 'poster records must use the poster format' });
    }
    if (record.type === 'presentation' && record.presentation?.format !== 'oral') {
      context.addIssue({ code: 'custom', path: ['presentation', 'format'], message: 'presentation records must use the oral format' });
    }
  })
});

const writings = defineCollection({
  loader: glob({ pattern: '**/*.{md,mdx}', base: './src/content/writings' }),
  schema: z.object({
    slug: z.string().regex(/^[a-z0-9]+(?:-[a-z0-9]+)*$/),
    title: z.string(),
    description: z.string(),
    date: z.coerce.date(),
    lang: z.enum(['en', 'ja']),
    kind: z.enum(['essay', 'note', 'literary', 'prose', 'fragment']),
    translationKey: z.string().optional(),
    publication: z.string().optional(),
    externalUrl: z.url().optional(),
    math: z.boolean().default(false),
    draft: z.boolean().default(true)
  })
});

const projects = defineCollection({
  loader: glob({ pattern: '**/*.{md,mdx}', base: './src/content/projects' }),
  schema: z.object({
    slug: z.string().regex(/^[a-z0-9]+(?:-[a-z0-9]+)*$/),
    title: z.string(),
    titleJa: z.string().optional(),
    summary: z.string(),
    summaryJa: z.string().optional(),
    status: z.enum(['active', 'completed', 'paused']),
    links: z.array(link).default([]),
    draft: z.boolean().default(true)
  })
});

export const collections = { updates, publications, writings, projects };
