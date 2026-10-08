export function GET({ site }: { site?: URL }) {
  const sitemap = site ? `\nSitemap: ${new URL(`${import.meta.env.BASE_URL}sitemap-index.xml`, site).href}\n` : '\n';
  return new Response(`User-agent: *\nAllow: /\n${sitemap}`, { headers: { 'Content-Type': 'text/plain; charset=utf-8' } });
}
