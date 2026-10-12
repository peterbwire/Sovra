// Reusable private Node transport for a Sovra SVR-HTTP-1 request handler.
import { createServer } from 'node:http';

function decodeResponse(output) {
    if (Buffer.byteLength(output, 'utf8') > 65536 || !output.endsWith('\n')) {
        throw new Error('Invalid Sovra response length');
    }
    const first = output.indexOf('\n');
    const second = output.indexOf('\n', first + 1);
    const third = output.indexOf('\n', second + 1);
    if (output.slice(0, first) !== 'SVR-HTTP-1' || second < 0 || third < 0) {
        throw new Error('Invalid Sovra response version');
    }
    const status = Number(output.slice(first + 1, second));
    const contentType = output.slice(second + 1, third);
    if (![200, 404].includes(status) || contentType !== 'text/plain') {
        throw new Error('Unsupported Sovra response metadata');
    }
    return { status, contentType: `${contentType}; charset=utf-8`, body: output.slice(third + 1, -1) };
}

export function createSovraHttpServer(program) {
    if (!program || typeof program.run !== 'function') throw new TypeError('Sovra program required');
    return createServer(async (request, response) => {
        try {
            const host = request.headers.host;
            const allowed = [`127.0.0.1:${request.socket.localPort}`, `localhost:${request.socket.localPort}`];
            if (!allowed.includes(host)) {
                response.writeHead(403).end('Forbidden');
                return;
            }
            if (request.url.length > 2048 || /[\u0000-\u001f\u007f]/u.test(request.url)) {
                response.writeHead(414).end('Path too long');
                return;
            }
            const chunks = [];
            let length = 0;
            for await (const chunk of request) {
                length += chunk.length;
                if (length > 4096) {
                    response.writeHead(413).end('Request too large');
                    return;
                }
                chunks.push(chunk);
            }
            let body;
            try { body = new TextDecoder('utf-8', { fatal: true }).decode(Buffer.concat(chunks, length)); }
            catch { response.writeHead(400).end('Invalid UTF-8'); return; }
            if (body.includes('\u0000')) { response.writeHead(400).end('Invalid request body'); return; }
            const result = program.run([request.method, request.url, body]);
            if (result.status !== 0 || result.stderr !== '') throw new Error('Sovra handler failed');
            const routed = decodeResponse(result.stdout);
            response.writeHead(routed.status, {
                'Content-Type': routed.contentType,
                'Cache-Control': 'no-store',
                'X-Content-Type-Options': 'nosniff',
            }).end(routed.body);
        } catch (error) {
            console.error(error);
            if (!response.headersSent) response.writeHead(500).end('Handler failed');
            else response.destroy();
        }
    });
}
