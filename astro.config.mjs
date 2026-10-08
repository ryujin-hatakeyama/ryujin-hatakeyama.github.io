import { defineConfig } from 'astro/config';
import sitemap from '@astrojs/sitemap';
import { unified } from '@astrojs/markdown-remark';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';

export default defineConfig({
  site: 'https://ryujin-hatakeyama.github.io/',
  output: 'static',
  trailingSlash: 'always',
  integrations: [sitemap()],
  markdown: {
    processor: unified({
      remarkPlugins: [remarkMath],
      rehypePlugins: [rehypeKatex],
      shikiConfig: {
        themes: {
          light: 'github-light',
          dark: 'github-dark'
        }
      }
    })
  },
  vite: {
    build: {
      cssMinify: true
    }
  }
});
