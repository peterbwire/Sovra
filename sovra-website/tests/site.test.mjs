import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile, access } from 'node:fs/promises';
import { runInNewContext } from 'node:vm';
import { createWebsiteServer } from '../server.mjs';

const root = new URL('../', import.meta.url);
const html = await readFile(new URL('index.html', root), 'utf8');

test('all page links, assets, and copy targets resolve', async () => {
  const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map(match => match[1]);
  assert.equal(new Set(ids).size, ids.length);
  for (const [, link] of html.matchAll(/(?:href|src)="([^"]+)"/g)) {
    if (link.startsWith('#')) {
      assert.ok(ids.includes(link.slice(1)), `Missing anchor: ${link}`);
    } else if (link.startsWith('https:')) {
      const prefix = 'https://github.com/peterbwire/Sovra';
      assert.ok(link === prefix || link.startsWith(prefix + '/blob/HEAD/'));
      if (link.includes('/blob/HEAD/')) {
        await access(new URL('../' + link.split('/blob/HEAD/')[1], root));
      }
    } else {
      await access(new URL(link, root));
    }
  }
  for (const [, id] of html.matchAll(/data-copy="([^"]+)"/g)) assert.ok(ids.includes(id));
  const displayed = html.match(/id="hello-source">([\s\S]*?)<\/code>/)[1]
    .replaceAll('&gt;', '>').replaceAll('&lt;', '<').replaceAll('&amp;', '&');
  assert.equal(displayed, (await readFile(new URL('examples/hello.svr', root), 'utf8')).trimEnd());
  assert.ok(!html.includes('curl ...'));
});

test('HTTP serves page and download, rejects invalid paths and methods', async t => {
  const server = createWebsiteServer();
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => new Promise(resolve => server.close(resolve)));
  const base = `http://127.0.0.1:${server.address().port}`;
  for (const path of ['/', '/styles.css', '/script.js', '/examples/hello.svr']) {
    const response = await fetch(base + path);
    assert.equal(response.status, 200);
    assert.ok((await response.text()).length > 0);
  }
  const head = await fetch(base + '/', { method: 'HEAD' });
  assert.equal(head.status, 200);
  assert.equal(await head.text(), '');
  assert.equal((await fetch(base + '/missing.html')).status, 404);
  assert.equal((await fetch(base + '/..%5c..%5cREADME.md')).status, 404);
  assert.equal((await fetch(base + '/%ZZ')).status, 400);
  assert.equal((await fetch(base + '/', { method: 'POST' })).status, 405);
});

function element() {
  const classes = new Set();
  return {
    handlers: {}, attributes: {}, hidden: true, textContent: '',
    classList: {
      add: name => classes.add(name), remove: name => classes.delete(name),
      contains: name => classes.has(name),
      toggle(name) { if (classes.has(name)) { classes.delete(name); return false; } classes.add(name); return true; },
    },
    addEventListener(name, handler) { this.handlers[name] = handler; },
    setAttribute(name, value) { this.attributes[name] = value; },
    contains(target) { return target === this; },
    focus() { this.focused = true; },
  };
}

test('navigation and clipboard success/failure remain usable', async () => {
  const menu = element(), nav = element(), link = element(), button = element();
  button.dataset = { copy: 'hello-source' };
  const ids = { 'hello-source': element(), 'copy-status': element(), year: element() };
  ids['hello-source'].textContent = 'fn main() {}';
  const document = element();
  document.documentElement = element();
  document.querySelector = selector => selector === '.menu-btn' ? menu : nav;
  document.querySelectorAll = selector => selector === '.nav a' ? [link] : [button];
  document.getElementById = id => ids[id];
  let selected, copied, resize;
  document.createRange = () => ({ selectNodeContents(source) { selected = source; } });
  const window = {
    matchMedia: () => ({ addEventListener(name, handler) { resize = handler; } }),
    getSelection: () => ({ removeAllRanges() {}, addRange() {} }),
  };
  const navigator = { clipboard: { async writeText(value) { copied = value; } } };
  runInNewContext(await readFile(new URL('script.js', root), 'utf8'), { document, window, navigator, Date });
  assert.equal(menu.hidden, false);
  menu.handlers.click();
  assert.equal(menu.attributes['aria-expanded'], 'true');
  document.handlers.keydown({ key: 'Escape' });
  assert.equal(menu.attributes['aria-expanded'], 'false');
  assert.equal(menu.focused, true);
  for (const close of [link.handlers.click, resize, () => document.handlers.click({ target: {} })]) {
    menu.handlers.click();
    close();
    assert.equal(nav.classList.contains('open'), false);
  }
  await button.handlers.click();
  assert.equal(copied, ids['hello-source'].textContent);
  assert.equal(ids['copy-status'].textContent, 'Copied to clipboard.');
  navigator.clipboard.writeText = async () => { throw new Error('Denied'); };
  await button.handlers.click();
  assert.equal(selected, ids['hello-source']);
  assert.match(ids['copy-status'].textContent, /Ctrl\+C or Command\+C/);
  assert.equal(ids.year.textContent, new Date().getFullYear());
});
