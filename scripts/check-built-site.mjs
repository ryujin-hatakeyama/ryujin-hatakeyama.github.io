import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const root = new URL('../dist/', import.meta.url);
const required = [
  'index.html', 'cv/index.html', 'research/index.html', 'writings/index.html', 'updates/index.html',
  'updates/research/index.html', 'updates/academia/index.html', 'updates/writing/index.html',
  '404.html', 'rss.xml', 'robots.txt', 'sitemap-index.xml', 'sitemap-0.xml'
];

const fail = (message) => {
  console.error(`Built-site check failed: ${message}`);
  process.exitCode = 1;
};

for (const path of required) {
  if (!existsSync(new URL(path, root))) fail(`missing ${path}`);
}

const files = [];
function walk(directory) {
  for (const name of readdirSync(directory)) {
    const path = join(directory, name);
    if (statSync(path).isDirectory()) walk(path);
    else files.push(path);
  }
}
walk(root.pathname);

const htmlFiles = files.filter((path) => path.endsWith('.html'));
const forbidden = [
  'Fixture award',
  'Test record',
  'private validation fixture',
  'NAME DETAILS',
  'AWAITING CONFIRMATION',
  'Verified entries will appear',
  'Affiliation details will be added',
  'Essays, notes, literary work, and other texts. Each piece remains in the language in which it was written unless an authorized translation exists.',
  'A reverse-chronological record of research, academic activities, and writing. Categories may overlap.',
  'My research concerns programming languages and their foundations.',
  'エッセイ、ノート、文芸作品、その他の文章。許可された翻訳がある場合を除き、それぞれ執筆された言語のまま掲載します。',
  '研究、学術・教育活動、執筆に関する記録を新しい順に掲載します。カテゴリーは重複することがあります。',
  'プログラミング言語とその基礎に関する研究をしています。'
];
for (const path of htmlFiles) {
  const html = readFileSync(path, 'utf8');
  if (relative(root.pathname, path).startsWith('ja/')) fail(`Japanese public route remains: ${relative(root.pathname, path)}`);
  if (html.includes('hreflang=')) fail(`${relative(root.pathname, path)} contains alternate-language metadata`);
  for (const phrase of forbidden) {
    if (html.includes(phrase)) fail(`${relative(root.pathname, path)} contains development fixture text: ${phrase}`);
  }
  for (const match of html.matchAll(/href="([^"]+)"/g)) {
    const href = match[1];
    if (/^(?:https?:|mailto:|#)/.test(href)) continue;
    const clean = href.split('#')[0].split('?')[0];
    if (!clean) continue;
    const normalized = clean.startsWith('/') ? clean.slice(1) : clean;
    const target = normalized.endsWith('/') ? `${normalized}index.html` : normalized;
    if (!existsSync(new URL(target, root))) fail(`${relative(root.pathname, path)} links to missing ${href}`);
  }
}

const home = readFileSync(new URL('index.html', root), 'utf8');
if (!home.includes('<link rel="canonical" href="https://ryujin-hatakeyama.github.io/">')) fail('homepage canonical URL is incorrect');
if (!home.includes('<meta property="og:url" content="https://ryujin-hatakeyama.github.io/">')) fail('homepage Open Graph URL is incorrect');
if (!home.includes('Hello!')) fail('English homepage does not contain the required greeting');
if (!home.includes("a second-year master's student")) fail('English homepage does not contain the M2 academic introduction');
if (!home.includes('Foundations of Software Science')) fail('English homepage is missing the research-group link');
if (!home.includes('Prof. Eijiro Sumii')) fail('English homepage is missing the supervisor link');
if (!home.includes('Oleg Kiselyov')) fail('English homepage is missing the collaborator link');
if (!home.includes('compositional descriptions of probabilistic models and staged inference code generation')) fail('English homepage is missing the current research description');
if (home.includes('compositional descriptions of hidden Markov models')) fail('English homepage still narrows the biography to hidden Markov models');
if (!home.includes('My broader interests lie in modal and categorical logic, and in the conditions of intelligibility of formal reasoning.')) fail('English homepage is missing the distinct broader interest');
if (!home.includes('appearance-toggle')) fail('appearance control is missing');
if (!home.includes('Ryujin Hatakeyama')) fail('English display name is missing');
if (!home.includes('畠山竜迅')) fail('Japanese name is missing or incorrectly spaced');
if (home.includes('畠山 竜迅')) fail('Japanese name contains an unwanted space');
if (!home.includes('はたけやま りゅうじん')) fail('Japanese name reading is missing');
if (!home.includes('hatakejama ɾʲɯːdʑiɴ')) fail('provisional segmental IPA is missing');
if (home.includes('<details')) fail('homepage pronunciation is hidden behind a disclosure');
if (home.includes('How to pronounce my name')) fail('homepage contains the old pronunciation toggle label');
if (home.includes('language-switch')) fail('public language switch remains');
if (!home.includes('Recent Updates')) fail('homepage is missing Recent Updates');
if (!home.includes('All updates')) fail('homepage is missing the updates archive link');
if (!home.includes('https://github.com/ryujin-hatakeyama')) fail('homepage is missing the verified GitHub profile');
if (home.includes('No updates yet.')) fail('English homepage exposes an empty News placeholder');
const academicListStart = home.indexOf('<ul class="home-academic-links">');
const academicListEnd = home.indexOf('</ul>', academicListStart);
const academicList = academicListStart === -1 || academicListEnd === -1
  ? ''
  : home.slice(academicListStart, academicListEnd);
if (!academicList) fail('English homepage academic-link list is missing');
const cvLinkPosition = academicList.indexOf('href="/cv/"');
if (cvLinkPosition === -1) fail('English homepage is missing the CV link');
if (academicList.includes('>Research</a>') || academicList.includes('>Publications</a>')) fail('Homepage contains a redundant Research or Publications link');
const presentationsPosition = academicList.indexOf('href="/research/#presentations"');
if (presentationsPosition < cvLinkPosition) fail('Talks & Presentations does not follow CV');
const githubPosition = academicList.indexOf('href="https://github.com/ryujin-hatakeyama"');
if (githubPosition < presentationsPosition) fail('GitHub does not follow Talks & Presentations');
const emailPosition = academicList.indexOf('href="mailto:');
if (emailPosition !== -1 && emailPosition < githubPosition) fail('Email does not follow GitHub');
if (home.indexOf('Recent Updates') < academicListEnd) fail('Recent Updates does not follow the homepage link list');
const publicHeader = home.slice(home.indexOf('<header class="site-header">'), home.indexOf('</header>'));
if (publicHeader.includes('>Writings</a>')) fail('Writings remains in primary navigation');

const research = readFileSync(new URL('research/index.html', root), 'utf8');
const removedResearchIntroduction = 'My current work uses staged computation to generate inference code from compositional descriptions of hidden Markov models.';
if (research.includes(removedResearchIntroduction)) fail('Research page contains the redundant introduction');
const jssstTitle = '代数的操作を用いた有限確率モデルの記述と多段階計算による推論コード生成';
const pplTitle = 'Applicativeによるグラフィカルモデルの表現と厳密推論に向けた実装';
for (const title of [jssstTitle, pplTitle]) {
  if (!research.includes(title)) fail(`Research page is missing presentation: ${title}`);
}
if (research.indexOf(jssstTitle) > research.indexOf(pplTitle)) fail('Research presentations are not ordered newest first');
if (!research.includes('The 43rd Annual Conference of the Japan Society for Software Science and Technology (JSSST 2026)')) fail('Research page is missing the JSSST 2026 venue');
if (!research.includes('Oral presentation · Unrefereed proceedings paper')) fail('JSSST record is missing its oral and unrefereed classifications');
if (research.includes('Refereed proceedings paper')) fail('A presentation is incorrectly presented as having a refereed proceedings paper');
if (!research.includes('https://jssst.or.jp/files/user/taikai/2026/papers/6b-1-R.pdf')) fail('JSSST record is missing its verified proceedings link');
if (!research.includes('The 27th Workshop on Programming and Programming Languages (PPL 2025)')) fail('Research page is missing the PPL 2025 venue');
if (!research.includes('Poster presentation (Category 3 / C3)')) fail('PPL record is missing its poster classification');
if (!research.includes('https://jssst-ppl.org/workshop/2025/accepted.html')) fail('PPL record is missing its verified official listing');
const publicationsStart = research.indexOf('<section id="publications"');
const publicationsEnd = research.indexOf('</section>', publicationsStart);
const publicationsSection = publicationsStart === -1 || publicationsEnd === -1 ? '' : research.slice(publicationsStart, publicationsEnd);
if (!publicationsSection) fail('Research page is missing the publications section');
if (publicationsSection.includes(jssstTitle) || publicationsSection.includes(pplTitle)) fail('Presentation appears in the publications section');

const cv = readFileSync(new URL('cv/index.html', root), 'utf8');
if (!cv.includes('Graduate School of Information Sciences')) fail('CV is missing the graduate school');
if (!cv.includes("second-year master's student (M2)")) fail('CV is missing the current M2 status');
if (!cv.includes("Master's program")) fail('CV is missing the current master’s program');
if (!cv.includes('Foundations of Software Science')) fail('CV is missing the research group');
if (!cv.includes('Prof. Eijiro Sumii')) fail('CV is missing the supervisor');
if (!cv.includes('2025–present')) fail('CV is missing the AIE participation dates');
if (!cv.includes('Selected participant')) fail('CV is missing the AIE participant status');
if (!cv.includes('WISE Program for AI Electronics (AIE)')) fail('CV is missing the AIE program name');
if (!cv.includes('https://www.aie.tohoku.ac.jp/english/')) fail('CV is missing the official AIE link');
for (const title of [jssstTitle, pplTitle]) {
  if (!cv.includes(title)) fail(`CV is missing presentation: ${title}`);
}
if (!cv.includes('Oral presentation · Unrefereed proceedings paper')) fail('CV is missing the JSSST presentation classification');
if (!cv.includes('Poster presentation (Category 3 / C3)')) fail('CV is missing the PPL poster classification');

const sitemap = readFileSync(new URL('sitemap-0.xml', root), 'utf8');
if (!sitemap.includes('https://ryujin-hatakeyama.github.io/')) fail('sitemap does not use the production origin');
if (sitemap.includes('/design-directions/') || sitemap.includes('/ja/')) fail('sitemap contains a development-only or disabled route');
if (sitemap.includes('https://ryujin-hatakeyama.github.io/ryujin-hatakeyama.github.io/')) {
  fail('sitemap contains an incorrect repository-name base prefix');
}
const robots = readFileSync(new URL('robots.txt', root), 'utf8');
if (!robots.includes('Sitemap: https://ryujin-hatakeyama.github.io/sitemap-index.xml')) fail('robots.txt has an incorrect sitemap URL');

if (!process.exitCode) console.log(`Built-site checks passed for ${htmlFiles.length} HTML files.`);
