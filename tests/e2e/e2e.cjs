// Uçtan uca test: gerçek uygulama + gerçek köprü + eklenti yüklü Chromium. tests/e2e/run.sh ile çalıştırın.
let pw;
try { pw = require('playwright'); } catch { pw = require(process.env.PLAYWRIGHT_MODULE || '/opt/node22/lib/node_modules/playwright'); }
const { chromium } = pw;
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const T = process.env.WORK || path.join(require('os').tmpdir(), 'idc-e2e');
fs.mkdirSync(T, { recursive: true });
const HOME = process.env.HOME;
const ROOT = path.resolve(__dirname, '../..');
const EXT = path.join(ROOT, 'extension/build/chromium');
const ID = 'khmbnekbmjoegchdepkfehmenkbfmbpd';
const sleep = (ms) => new Promise(r => setTimeout(r, ms));
const mk = (n) => { const b = Buffer.alloc(n); for (let i = 0; i < n; i++) b[i] = (i * 31 + 7) % 251; return b; };
const sha = (b) => crypto.createHash('sha256').update(b).digest('hex');
function find(dir, name) {
  if (!fs.existsSync(dir)) return null;
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) { const r = find(p, name); if (r) return r; } else if (e.name === name) return p;
  }
  return null;
}
(async () => {
  const udd = path.join(T, 'profile');
  fs.rmSync(udd, { recursive: true, force: true });
  // Chromium kullanıcı düzeyindeki köprü bildirimlerini <user-data-dir>/NativeMessagingHosts altında arar.
  fs.mkdirSync(path.join(udd, 'NativeMessagingHosts'), { recursive: true });
  fs.copyFileSync(path.join(HOME, '.config/chromium/NativeMessagingHosts/com.furkanyildirim.idc.json'), path.join(udd, 'NativeMessagingHosts/com.furkanyildirim.idc.json'));
  const ctx = await chromium.launchPersistentContext(udd, {
    headless: false, executablePath: process.env.CHROME || undefined, acceptDownloads: true,
    downloadsPath: path.join(T, 'browser-dl'),
    args: [`--disable-extensions-except=${EXT}`, `--load-extension=${EXT}`, '--no-first-run'], viewport: { width: 900, height: 600 },
  });
  let [sw] = ctx.serviceWorkers();
  if (!sw) sw = await ctx.waitForEvent('serviceworker', { timeout: 15000 });
  await sleep(3000);
  const st = await sw.evaluate(async () => (await chrome.storage.local.get(null)));
  console.log('HELLO appOk=', st.appOk, 'appVersion=', st.appVersion);
  const page = await ctx.newPage();
  await page.goto('http://127.0.0.1:8766/page.html');
  await sleep(500);
  const results = {};
  for (const [sel, name] of [['#a', 'file.zip'], ['#b', 'secure.zip'], ['#c', 'once.zip']]) {
    await page.click(sel);
    await sleep(4000);
  }
  // Uygulamanın indirmelerini bekle.
  for (let i = 0; i < 60; i++) {
    const a = find(path.join(HOME, 'Downloads'), 'file.zip'), b = find(path.join(HOME, 'Downloads'), 'secure.zip');
    if (a && b) break;
    await sleep(1000);
  }
  const want = { 'file.zip': mk(8 * 1024 * 1024 + 333), 'secure.zip': mk(3 * 1024 * 1024) };
  for (const n of Object.keys(want)) {
    const p = find(path.join(HOME, 'Downloads'), n);
    results[n] = p ? (sha(fs.readFileSync(p)) === sha(want[n]) ? 'IDC OK ' + p.replace(HOME, '~') : 'IDC BOZUK') : 'IDC YOK';
  }
  const items = await sw.evaluate(async () => (await chrome.downloads.search({})).map(i => ({ url: i.url.split('/').pop(), state: i.state, file: i.filename.split('/').pop(), bytes: i.bytesReceived })));
  console.log('Tarayıcı indirme listesi:', JSON.stringify(items));
  const once = items.find(i => i.url === 'once.zip');
  results['once.zip'] = once && once.state === 'complete' ? 'TARAYICIYA DEVREDİLDİ ve tamamlandı (' + once.bytes + ' bayt)' : 'once: ' + JSON.stringify(once);
  console.log(JSON.stringify(results, null, 2));
  await ctx.close();
})().catch(e => { console.error(e); process.exit(1); });
