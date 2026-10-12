// Launch the private HTTP adapter with the real Sovra package.
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createSovraProgram } from '../../01-task-manager/gui/bridge.mjs';
import { createSovraHttpServer } from './adapter.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const options = {
    port: '8789', engine: 'javascript',
    compiler: resolve(here, '../../../target/debug', process.platform === 'win32' ? 'svr.exe' : 'svr'),
    source: resolve(here, '../app'),
};
for (let index = 2; index < process.argv.length; index += 2) {
    const key = process.argv[index]?.slice(2);
    if (!Object.hasOwn(options, key) || !process.argv[index + 1]) {
        throw new Error('Usage: server.mjs [--compiler PATH] [--engine javascript|interpreter] [--port PORT]');
    }
    options[key] = process.argv[index + 1];
}
const port = Number(options.port);
if (!Number.isInteger(port) || port < 0 || port > 65535) throw new Error('Invalid port');
const program = createSovraProgram(options);
const server = createSovraHttpServer(program);
server.listen(port, '127.0.0.1', () => {
    console.log(`Sovra HTTP Server (${options.engine}): http://127.0.0.1:${server.address().port}/`);
});
server.on('close', () => program.close());
