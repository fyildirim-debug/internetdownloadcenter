#!/usr/bin/env node
// IDC eklentisini Chromium (MV3 servis çalışanı) ve Firefox (MV3 olay sayfası) için derler.
// Bağımlılık gerektirmez: node extension/build.mjs
//
// Çıktı:
//   build/chromium/        Chrome, Edge, Brave, Opera, Vivaldi için paketlenmemiş eklenti
//   build/firefox/         Firefox, Zen, LibreWolf, Floorp için
//   build/idc-chromium.zip
//   build/idc-firefox.xpi

import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { fileURLToPath } from 'node:url';

const ROOT = path.dirname(fileURLToPath(import.meta.url));
const SRC = path.join(ROOT, 'src');
const OUT = path.join(ROOT, 'build');
const IDS = JSON.parse(fs.readFileSync(path.join(ROOT, 'ids.json'), 'utf8'));

function readVersion() {
  // Sürüm çalışma alanındaki Cargo.toml'dan okunur; bulunamazsa 1.0.0.
  try {
    const cargo = fs.readFileSync(path.join(ROOT, '..', 'Cargo.toml'), 'utf8');
    const m = /\[workspace\.package\][^[]*?version\s*=\s*"([^"]+)"/s.exec(cargo);
    if (m) return m[1].replace(/[^0-9.]/g, '') || '1.0.0';
  } catch (_) {
    /* varsayılan */
  }
  return '1.0.0';
}

const VERSION = process.env.IDC_VERSION || readVersion();
const BACKGROUND_PARTS = ['compat.js', 'protocol.js', 'rules.js', 'background.js'];

function icons(prefix = 'icon') {
  return Object.fromEntries([16, 32, 48, 128].map((s) => [String(s), `icons/${prefix}-${s}.png`]));
}

function baseManifest() {
  return {
    manifest_version: 3,
    name: '__MSG_extName__',
    short_name: 'IDC',
    version: VERSION,
    description: '__MSG_extDesc__',
    default_locale: 'en',
    homepage_url: 'https://github.com/fyildirim-debug/internetdownloadcenter',
    icons: icons(),
    action: { default_popup: 'popup/popup.html', default_icon: icons(), default_title: '__MSG_extName__' },
    permissions: ['downloads', 'nativeMessaging', 'cookies', 'webRequest', 'contextMenus', 'storage', 'tabs', 'alarms', 'notifications'],
    host_permissions: ['<all_urls>'],
    content_scripts: [
      {
        matches: ['http://*/*', 'https://*/*'],
        js: ['content.js'],
        all_frames: true,
        run_at: 'document_start',
      },
    ],
  };
}

function chromiumManifest() {
  const m = baseManifest();
  m.key = fs.readFileSync(path.join(SRC, 'chromium-key.b64'), 'utf8').trim();
  m.minimum_chrome_version = '116';
  m.background = { service_worker: 'background.js' };
  // Gizli pencerelerin kendi arka planı olsun; çerezler doğru depodan okunur.
  m.incognito = 'split';
  m.content_scripts[0].match_about_blank = true;
  return m;
}

function firefoxManifest() {
  const m = baseManifest();
  m.background = { scripts: ['background.js'] };
  m.permissions.push('webRequestBlocking');
  m.browser_specific_settings = {
    gecko: {
      id: IDS.gecko,
      strict_min_version: '128.0',
      data_collection_permissions: { required: ['none'] },
    },
  };
  return m;
}

function rmrf(p) {
  fs.rmSync(p, { recursive: true, force: true });
}

function copyDir(from, to, filter = () => true) {
  fs.mkdirSync(to, { recursive: true });
  for (const ent of fs.readdirSync(from, { withFileTypes: true })) {
    const a = path.join(from, ent.name);
    const b = path.join(to, ent.name);
    if (!filter(a, ent)) continue;
    if (ent.isDirectory()) copyDir(a, b, filter);
    else fs.copyFileSync(a, b);
  }
}

function buildTarget(name, manifest) {
  const dir = path.join(OUT, name);
  rmrf(dir);
  fs.mkdirSync(dir, { recursive: true });
  const bg = BACKGROUND_PARTS.map((f) => `// ---- ${f}\n` + fs.readFileSync(path.join(SRC, f), 'utf8')).join('\n');
  fs.writeFileSync(path.join(dir, 'background.js'), `'use strict';\n${bg}`);
  fs.copyFileSync(path.join(SRC, 'content.js'), path.join(dir, 'content.js'));
  fs.copyFileSync(path.join(SRC, 'compat.js'), path.join(dir, 'compat.js'));
  copyDir(path.join(SRC, 'popup'), path.join(dir, 'popup'));
  copyDir(path.join(SRC, 'icons'), path.join(dir, 'icons'), (p) => p.endsWith('.png') || fs.statSync(p).isDirectory());
  copyDir(path.join(ROOT, '_locales'), path.join(dir, '_locales'));
  fs.writeFileSync(path.join(dir, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
  return dir;
}

// ------------------------------------------------------------ küçük zip yazıcı (store + deflate)

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function listFiles(dir, base = dir) {
  const out = [];
  for (const ent of fs.readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    const p = path.join(dir, ent.name);
    if (ent.isDirectory()) out.push(...listFiles(p, base));
    else out.push(path.relative(base, p).split(path.sep).join('/'));
  }
  return out;
}

function writeZip(dir, zipPath) {
  const locals = [];
  const centrals = [];
  let offset = 0;
  // Sabit tarih: aynı kaynak aynı arşivi üretsin (1 Ocak 2026 00:00).
  const dosTime = 0;
  const dosDate = ((2026 - 1980) << 9) | (1 << 5) | 1;
  for (const rel of listFiles(dir)) {
    const data = fs.readFileSync(path.join(dir, rel));
    const deflated = zlib.deflateRawSync(data, { level: 9 });
    const useDeflate = deflated.length < data.length;
    const body = useDeflate ? deflated : data;
    const name = Buffer.from(rel, 'utf8');
    const crc = crc32(data);
    const lh = Buffer.alloc(30);
    lh.writeUInt32LE(0x04034b50, 0);
    lh.writeUInt16LE(20, 4);
    lh.writeUInt16LE(0x0800, 6); // UTF-8 ad
    lh.writeUInt16LE(useDeflate ? 8 : 0, 8);
    lh.writeUInt16LE(dosTime, 10);
    lh.writeUInt16LE(dosDate, 12);
    lh.writeUInt32LE(crc, 14);
    lh.writeUInt32LE(body.length, 18);
    lh.writeUInt32LE(data.length, 22);
    lh.writeUInt16LE(name.length, 26);
    lh.writeUInt16LE(0, 28);
    locals.push(lh, name, body);
    const ch = Buffer.alloc(46);
    ch.writeUInt32LE(0x02014b50, 0);
    ch.writeUInt16LE(20, 4);
    ch.writeUInt16LE(20, 6);
    ch.writeUInt16LE(0x0800, 8);
    ch.writeUInt16LE(useDeflate ? 8 : 0, 10);
    ch.writeUInt16LE(dosTime, 12);
    ch.writeUInt16LE(dosDate, 14);
    ch.writeUInt32LE(crc, 16);
    ch.writeUInt32LE(body.length, 20);
    ch.writeUInt32LE(data.length, 24);
    ch.writeUInt16LE(name.length, 28);
    ch.writeUInt32LE(0, 38);
    ch.writeUInt32LE(offset, 42);
    centrals.push(ch, name);
    offset += lh.length + name.length + body.length;
  }
  const cd = Buffer.concat(centrals);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  const count = centrals.length / 2;
  end.writeUInt16LE(count, 8);
  end.writeUInt16LE(count, 10);
  end.writeUInt32LE(cd.length, 12);
  end.writeUInt32LE(offset, 16);
  fs.writeFileSync(zipPath, Buffer.concat([...locals, cd, end]));
}

// ------------------------------------------------------------ ana akış

fs.mkdirSync(OUT, { recursive: true });
const chromium = buildTarget('chromium', chromiumManifest());
const firefox = buildTarget('firefox', firefoxManifest());
writeZip(chromium, path.join(OUT, 'idc-chromium.zip'));
writeZip(firefox, path.join(OUT, 'idc-firefox.xpi'));
console.log(`IDC eklentisi ${VERSION} derlendi:`);
console.log('  ' + path.relative(process.cwd(), chromium) + '  (kimlik ' + IDS.chromium + ')');
console.log('  ' + path.relative(process.cwd(), firefox) + '  (kimlik ' + IDS.gecko + ')');
console.log('  ' + path.relative(process.cwd(), path.join(OUT, 'idc-chromium.zip')));
console.log('  ' + path.relative(process.cwd(), path.join(OUT, 'idc-firefox.xpi')));
