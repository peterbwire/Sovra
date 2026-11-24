export const MAX_MESSAGE = 2 * 1024 * 1024;
const MAX_HEADER = 4096;
const utf8 = new TextDecoder('utf-8', { fatal: true });

export function frame(message) {
    const body = Buffer.from(JSON.stringify(message), 'utf8');
    if (body.length > MAX_MESSAGE) throw new Error('LSP message exceeds 2 MiB');
    return Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`), body]);
}

// Length counts UTF-8 bytes, including when headers or code points span chunks.
export class FrameReader {
    buffer = Buffer.alloc(0);
    length = undefined;
    constructor(receive) { this.receive = receive; }
    push(chunk) {
        if (this.buffer.length + chunk.length > MAX_MESSAGE * 2 + MAX_HEADER) throw new Error('LSP input buffer exceeded');
        this.buffer = Buffer.concat([this.buffer, chunk]);
        while (true) {
            if (this.length === undefined) {
                const end = this.buffer.indexOf('\r\n\r\n');
                if (end < 0) { if (this.buffer.length > MAX_HEADER) throw new Error('LSP header exceeded'); return; }
                if (end > MAX_HEADER) throw new Error('LSP header exceeded');
                const header = this.buffer.subarray(0, end);
                if ([...header].some(byte => byte > 126 || (byte < 32 && ![9, 10, 13].includes(byte)))) throw new Error('Invalid LSP header byte');
                const lengths = [];
                for (const line of header.toString('ascii').split('\r\n')) {
                    const pair = /^([A-Za-z-]+):[ \t]*(.*)$/.exec(line);
                    if (!pair) throw new Error('Malformed LSP header');
                    if (pair[1].toLowerCase() === 'content-length') lengths.push(pair[2]);
                }
                if (lengths.length !== 1 || !/^[0-9]{1,8}$/.test(lengths[0])) throw new Error('Invalid LSP Content-Length');
                this.length = Number(lengths[0]);
                if (!this.length || this.length > MAX_MESSAGE) throw new Error('LSP body exceeds limit');
                this.buffer = this.buffer.subarray(end + 4);
            }
            if (this.buffer.length < this.length) return;
            const body = this.buffer.subarray(0, this.length);
            this.buffer = this.buffer.subarray(this.length);
            this.length = undefined;
            // Framing errors are fatal; known-length invalid JSON is recoverable.
            let message;
            try { message = JSON.parse(utf8.decode(body)); }
            catch { this.receive(undefined); continue; }
            this.receive(message);
        }
    }
}
