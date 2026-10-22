// Aralık destekli, çerez denetimli ve tek kullanımlık bağlantılı test sunucusu.
const http = require('http');
const mk = (n) => { const b = Buffer.alloc(n); for (let i = 0; i < n; i++) b[i] = (i * 31 + 7) % 251; return b; };
const files = { '/file.zip': mk(8 * 1024 * 1024 + 333), '/secure.zip': mk(3 * 1024 * 1024), '/once.zip': mk(2 * 1024 * 1024) };
let onceUsed = 0;
const log = [];
function send(req, res, name, buf, extra) {
  const range = /bytes=(\d+)-(\d*)/.exec(req.headers.range || '');
  const h = Object.assign({ 'Content-Type': 'application/zip', 'Accept-Ranges': 'bytes', 'Content-Disposition': `attachment; filename="${name}"` }, extra || {});
  let a = 0, b = buf.length - 1, code = 200;
  if (range) { a = +range[1]; if (range[2]) b = Math.min(+range[2], b); code = 206; h['Content-Range'] = `bytes ${a}-${b}/${buf.length}`; }
  h['Content-Length'] = b - a + 1;
  res.writeHead(code, h);
  const part = buf.subarray(a, b + 1);
  let off = 0;
  const iv = setInterval(() => { if (res.destroyed) return clearInterval(iv); if (off >= part.length) { clearInterval(iv); return res.end(); } res.write(part.subarray(off, off + 32768)); off += 32768; }, +(process.env.DELAY || 8));
}
http.createServer((req, res) => {
  log.push({ url: req.url, range: req.headers.range || null, cookie: req.headers.cookie || null, ua: (req.headers['user-agent'] || '').slice(0, 40) });
  if (req.url === '/log') { res.writeHead(200, { 'Content-Type': 'application/json' }); return res.end(JSON.stringify(log)); }
  if (req.url.startsWith('/page')) {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8', 'Set-Cookie': 'sess=abc123; HttpOnly; Path=/' });
    return res.end(`<!doctype html><title>IDC E2E</title><body><a id="a" href="/file.zip">file</a> <a id="b" href="/secure.zip">secure</a> <a id="c" href="/once.zip">once</a></body>`);
  }
  if (req.url.startsWith('/secure.zip')) {
    if (!(req.headers.cookie || '').includes('sess=abc123')) { res.writeHead(403, { 'Content-Type': 'text/html' }); return res.end('<h1>Giriş gerekli</h1>'); }
    return send(req, res, 'secure.zip', files['/secure.zip']);
  }
  if (req.url.startsWith('/once.zip')) {
    // İlk istek (tarayıcınınki) geçer, sonrakiler engellenir: indirme yöneticisi engelleyen site.
    if (onceUsed++ > 0) { res.writeHead(403, { 'Content-Type': 'text/html' }); return res.end('<h1>Download managers are not allowed</h1>'); }
    return send(req, res, 'once.zip', files['/once.zip']);
  }
  if (files[req.url]) return send(req, res, req.url.slice(1), files[req.url]);
  res.writeHead(404); res.end();
}).listen(8766, '127.0.0.1', () => console.log('listening'));
