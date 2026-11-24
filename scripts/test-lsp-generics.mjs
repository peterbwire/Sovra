// Actual compiler/LSP integration: explicit dialect, UTF-16, pure formatting and retirement.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { mkdtemp, mkdir, writeFile, readFile, readdir, rm, realpath } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { FrameReader, frame } from '../toolchain/ide/language-server/transport.mjs';
import { canonicalFile, decodeContext, decodeGenericTaskContext, packageDiagnostics, overlayFrame } from '../toolchain/ide/language-server/packages.mjs';
import { prepareTaskTarget } from '../toolchain/ide/task-targets.mjs';
import { decodePackageNavigation } from '../toolchain/ide/language-server/package-navigation.mjs';
const compiler = resolve(process.env.SOVRA_TEST_COMPILER || `target/debug/svr${process.platform === 'win32' ? '.exe' : ''}`);
const scratch = await mkdtemp(join(tmpdir(), 'sovra-lsp-generics-proof-'));
const before = new Set((await readdir(tmpdir())).filter(name => name.startsWith('sovra-lsp-format-')));
const root = join(scratch, 'broken package'); await mkdir(join(root, 'src'), { recursive: true });
await writeFile(join(root, 'sovra.toml'), '[project]\nname="bad"\nentry="src/main.svr"\n[dependencies.missing]\npath="../absent"\n');
const file = join(root, 'src/main.svr'); const disk = 'fn main() {}'; await writeFile(file, disk);
const uri = pathToFileURL(file).href;
const marker = join(scratch, 'never-created');
function client() {
    const child = spawn(process.execPath, ['toolchain/ide/language-server/server.mjs', compiler], { stdio: ['pipe', 'pipe', 'pipe'] });
    const queued = [], pending = []; let stderr = '', ended = false, exitCode;
    child.stderr.on('data', chunk => { stderr += chunk; });
    const closed = new Promise(done => child.on('close', code => { ended = true; exitCode = code; done(code); }));
    child.stdin.on('error', () => {});
    const reader = new FrameReader(message => {
        const index = pending.findIndex(waiter => waiter.predicate(message));
        if (index < 0) queued.push(message);
        else { const waiter = pending.splice(index, 1)[0]; clearTimeout(waiter.timer); waiter.accept(message); }
    });
    child.stdout.on('data', chunk => reader.push(chunk));
    const wait = (predicate, label) => new Promise((accept, reject) => {
        const index = queued.findIndex(predicate); if (index >= 0) return accept(queued.splice(index, 1)[0]);
        const record = { predicate, accept };
        record.timer = setTimeout(() => { pending.splice(pending.indexOf(record), 1); reject(new Error(`LSP generic timeout ${label}; exited=${ended}/${exitCode}: ${stderr}; ${JSON.stringify(queued)}`)); }, 20000); pending.push(record);
    });
    const message = (method, params, id) => ({ jsonrpc: '2.0', method, params, ...(id === undefined ? {} : { id }) });
    return { child, queued, closed, wait,
        send(method, params, id) { child.stdin.write(frame(message(method, params, id))); },
        batch(items) { child.stdin.write(Buffer.concat(items.map(([method, params, id]) => frame(message(method, params, id))))); },
        response(id) { return wait(value => value.id === id, `response ${id}`); },
        diagnostic(version) { return wait(value => value.method === 'textDocument/publishDiagnostics' && value.params.uri === uri && value.params.version === version, `diagnostic ${version}`); },
        async stop() { if (!ended) { this.send('shutdown', null, 990); assert.equal((await this.response(990)).result, null); this.send('exit'); assert.equal(await closed, 0); } },
        kill() { if (!ended) child.kill(); for (const item of pending) clearTimeout(item.timer); },
    };
}
let c;
try {
    c = client();
    c.send('initialize', { initializationOptions: { experimentalData: true } }, 998);
    assert.equal((await c.response(998)).error.code, -32602);
    c.send('initialize', { initializationOptions: { experimentalGenerics: 'true' } }, 1);
    assert.equal((await c.response(1)).error.code, -32602);
    c.send('initialize', { initializationOptions: { experimentalV2: true, experimentalGenerics: true } }, 2);
    assert.equal((await c.response(2)).error.code, -32602);
    c.send('initialize', { rootUri: pathToFileURL(root).href, initializationOptions: { experimentalGenerics: true } }, 3);
    const caps = (await c.response(3)).result.capabilities;
    assert.equal(caps.experimental.sovraPipeline, 'generics-standalone');
    for (const capability of ['documentFormattingProvider']) assert.equal(caps[capability], true);
    assert.equal(caps.definitionProvider,true);assert.equal(caps.hoverProvider,true); assert.equal(caps.documentSymbolProvider,true);
    c.send('initialized', {});
    const invalid = '// λ😀\r\n' + 'fn bad(x:std::Option<Int>)->Int{let label="λ😀";let n=x?;return 1;}fn main(){}';
    c.send('textDocument/didOpen', { textDocument: { uri, languageId: 'sovra', version: 1, text: invalid } });
    const diagnostic = (await c.diagnostic(1)).params.diagnostics.find(item => item.code === 'E3123');
    assert.ok(diagnostic);
    const line = invalid.split('\n').at(-1), question = line.indexOf('?');
    assert.deepEqual(diagnostic.range, { start: { line: 1, character: question }, end: { line: 1, character: question + 1 } });
    c.send('textDocument/documentSymbol',{textDocument:{uri}},9);assert.deepEqual((await c.response(9)).result,[]);
    const valid = 'fn pick<T>(x:std::Option<T>)->std::Option<T>{return std::Option::<T>::Some(x?);}fn main(){let label="λ😀";let value=pick::<Int>(std::Option::<Int>::Some(1));let never_executed=1/0;}';
    c.send('textDocument/didChange', { textDocument: { uri, version: 2 }, contentChanges: [{ text: valid }] });
    assert.deepEqual((await c.diagnostic(2)).params.diagnostics, []);
    c.send('textDocument/hover', { textDocument: { uri }, position: { line: 0, character: 5 } }, 10);
    assert.equal((await c.response(10)).result.contents.value,'fn pick<T>(x:std::Option<T>)->std::Option<T>');
    c.send('textDocument/definition', { textDocument: { uri }, position: { line: 0, character: valid.indexOf('pick::<Int>')+1 } }, 11);
    const definition=(await c.response(11)).result;assert.equal(definition.uri,uri);assert.deepEqual(definition.range,{start:{line:0,character:3},end:{line:0,character:7}});
    c.send('textDocument/hover',{textDocument:{uri},position:{line:0,character:valid.indexOf('pick::<Int>')+1}},17);assert.equal((await c.response(17)).result.contents.value,'fn pick<T>(x:std::Option<T>)->std::Option<T>');
    c.send('textDocument/definition',{textDocument:{uri},position:{line:0,character:valid.indexOf('x?')}},18);
    assert.deepEqual((await c.response(18)).result.range,{start:{line:0,character:valid.indexOf('(x:')+1},end:{line:0,character:valid.indexOf('(x:')+2}});
    c.send('textDocument/hover',{textDocument:{uri},position:{line:0,character:valid.indexOf('x?')}},19);
    assert.equal((await c.response(19)).result.contents.value,'parameter x: std::Option<T>');
    c.send('textDocument/documentSymbol', { textDocument: { uri } }, 12);
    assert.deepEqual((await c.response(12)).result.map(item=>item.name),['pick','main']);
    c.send('textDocument/formatting', { textDocument: { uri } }, 13);
    const edits = (await c.response(13)).result;
    assert.equal(edits.length, 1); assert.ok(edits[0].newText.includes('x?'));
    assert.deepEqual(edits[0].range, { start: { line: 0, character: 0 }, end: { line: 0, character: valid.length } });
    assert.equal(await readFile(file, 'utf8'), disk); // Returns edits, never writes the workspace.

    c.send('sovra/taskTarget', { textDocument: { uri, version: 2 } }, 14);
    const target = (await c.response(14)).result;
    assert.equal(target.package, false); assert.equal(target.target, await realpath(file)); assert.deepEqual(target.manifests, []);
    c.send('textDocument/completion', { textDocument: { uri }, position: { line: 0, character: 0 } }, 15);
    const suggestions = (await c.response(15)).result.map(item => item.label);
    assert.ok(suggestions.includes('enum')); assert.ok(suggestions.includes('match')); assert.ok(!suggestions.includes('mod'));
    c.batch([
        ['textDocument/didChange', { textDocument: { uri, version: 3 }, contentChanges: [{ text: invalid }] }],
        ['textDocument/didChange', { textDocument: { uri, version: 4 }, contentChanges: [{ text: valid }] }],
    ]);
    assert.deepEqual((await c.diagnostic(4)).params.diagnostics, []);
    const typed='// λ😀\r\nstruct Box<T>{value:T}type Alias<T>=Box<T>;fn unused<T>(x:Alias<T>)->Box<T>{return x;}fn main(){}';
    c.send('textDocument/didChange',{textDocument:{uri,version:5},contentChanges:[{text:typed}]});
    assert.deepEqual((await c.diagnostic(5)).params.diagnostics,[]);
    const typedLine=typed.split('\n')[1];
    for(const [id,token,offset,target,label] of [[21,'x:Alias',2,'Alias','type Alias<T>=Box<T>;'],[23,'->Box',2,'Box','struct Box<T>']]) {
        const position={line:1,character:typedLine.indexOf(token)+offset};
        c.send('textDocument/definition',{textDocument:{uri},position},id);
        const got=(await c.response(id)).result;assert.equal(got.uri,uri);
        assert.deepEqual(got.range,{start:{line:1,character:typedLine.indexOf(target+'<')},end:{line:1,character:typedLine.indexOf(target+'<')+target.length}});
        c.send('textDocument/hover',{textDocument:{uri},position},id+1);assert.equal((await c.response(id+1)).result.contents.value,label);
    }
    c.send('textDocument/didChange',{textDocument:{uri,version:6},contentChanges:[{text:'fn main('+' '.repeat(4200)+'){}'}]});
    assert.deepEqual((await c.diagnostic(6)).params.diagnostics,[]);
    c.send('textDocument/documentSymbol',{textDocument:{uri}},16);assert.deepEqual((await c.response(16)).result,[]);
    c.batch([
        ['textDocument/formatting', { textDocument: { uri } }, 30],
        ['textDocument/didChange', { textDocument: { uri, version: 7 }, contentChanges: [{ text: valid }] }],
    ]);
    assert.equal((await c.response(30)).error.code, -32801);
    assert.deepEqual((await c.diagnostic(7)).params.diagnostics, []);
    c.send('textDocument/didClose', { textDocument: { uri } });
    await c.stop(); c = undefined;
    assert.equal(await readFile(file, 'utf8'), disk);
    await assert.rejects(readFile(marker));
    c = client(); c.send('initialize', { capabilities: {} }, 20);
    assert.equal((await c.response(20)).result.capabilities.experimental.sovraPipeline, 'legacy');
    c.send('textDocument/didOpen', { textDocument: { uri, languageId: 'sovra', version: 1, text: valid } });
    assert.ok((await c.diagnostic(1)).params.diagnostics.length > 0);
    await c.stop(); c = undefined;
    if (process.platform === 'linux' && ['x64', 'ia32', 'arm64'].includes(process.arch)) {
        const project = join(scratch, 'generic package tasks'), app = join(project, 'app'), helper = join(project, 'helper');
        await mkdir(app, { recursive: true }); await mkdir(helper);
        const manifestPath = join(app, 'sovra.toml');
        const manifestText = '[project]\nname="app"\nentry="main.svr"\n[modules]\nown="own.svr"\n[dependencies.helper]\npath="../helper"\n';
        await writeFile(manifestPath, manifestText);
        await writeFile(join(helper, 'sovra.toml'), '[project]\nname="helper"\nentry="main.svr"\n[modules]\nvalues="values.svr"\n');
        await writeFile(join(helper, 'main.svr'), 'fn main(){}');
        const mainPath=join(app,'main.svr'), providerPath=join(helper,'values.svr'), ownPath=join(app,'own.svr'), notesPath=join(app,'notes.svr');
        await writeFile(mainPath,'fn main(){}'); await writeFile(providerPath,'intentionally invalid until saved');
        await writeFile(ownPath,'mod own {export fn value()->Int{return 3;}}'); await writeFile(notesPath,'fn main(){}');
        const mainUri=pathToFileURL(mainPath).href;
        c=client();
        c.send('initialize',{initializationOptions:{experimentalGenericPackageTasks:true}},40);
        assert.equal((await c.response(40)).error.code,-32602);
        c.send('initialize',{initializationOptions:{experimentalGenerics:true,experimentalGenericPackageTasks:'true'}},41);
        assert.equal((await c.response(41)).error.code,-32602);
        c.send('initialize',{rootUri:pathToFileURL(project).href,initializationOptions:{experimentalGenerics:true,experimentalGenericPackageTasks:true}},42);
        const capabilities=(await c.response(42)).result.capabilities;
        assert.equal(capabilities.experimental.sovraGenericPackageTasks,true);
        assert.equal(capabilities.experimental.sovraPipeline,'generics-standalone');
        const document=(path,text,dirty=false)=>({uri:{scheme:'file',toString:()=>pathToFileURL(path).href},text,isDirty:dirty,version:1,saves:0,async save(){await writeFile(new URL(this.uri.toString()),this.text);this.isDirty=false;this.saves++;return true;}});
        const active=document(mainPath,'use helper::values;fn main(){let value=helper::values::id::<Int>(42);if value!=42{let failure=1/0;}}',true);
        const provider=document(providerPath,'mod values {export fn id<T>(value:T)->T{return value;}}',true);
        const notes=document(notesPath,'fn main(){print(99);}',true), manifest=document(manifestPath,manifestText);
        const open=[active,provider,notes,manifest];
        c.send('textDocument/didOpen',{textDocument:{uri:mainUri,languageId:'sovra',version:1,text:active.text}});
        let requestId=100;
        const resolveTarget=async doc=>{
            const id=requestId++;c.send('sovra/taskTarget',{textDocument:{uri:doc.uri.toString(),version:doc.version}},id);
            const reply=await c.response(id);if(reply.error)throw Object.assign(new Error(reply.error.message),{code:reply.error.code});return reply.result;
        };
        const prepare=()=>prepareTaskTarget(active,()=>open,resolveTarget,canonicalFile,async()=>{});
        const plan=await prepare();
        assert.equal(plan.package,true);assert.equal(plan.target,await realpath(app));assert.equal(plan.sources.length,4);assert.equal(plan.manifests.length,2);
        assert.ok(plan.sources.includes(await realpath(providerPath)));assert.ok(plan.sources.includes(await realpath(ownPath)));
        assert.equal(active.saves,1);assert.equal(provider.saves,1);assert.equal(notes.saves,0);assert.equal(notes.isDirty,true);
        for(const verb of ['check','run']){
            const result=spawnSync(compiler,[verb,'--experimental-generics',plan.target],{cwd:plan.cwd,encoding:'utf8',timeout:15000});
            assert.equal(result.status,0,result.stdout+result.stderr);
        }
        assert.equal(await readFile(mainPath,'utf8'),active.text);
        await writeFile(mainPath,active.text.replace('value!=42','value!=43'));
        assert.equal(spawnSync(compiler,['run','--experimental-generics',plan.target],{encoding:'utf8',timeout:15000}).status,1);
        await writeFile(mainPath,active.text);
        manifest.isDirty=true;await assert.rejects(prepare(),/Save package manifests/);assert.equal(manifest.saves,0);manifest.isDirty=false;
        const rawResult=spawnSync(compiler,['check','--experimental-generics','--format','json','--package-context',app],{encoding:'utf8',timeout:15000});
        assert.equal(rawResult.status,0,rawResult.stderr);const raw=JSON.parse(rawResult.stdout);
        assert.equal(decodeGenericTaskContext(raw,plan.target).size,4);assert.throws(()=>decodeContext(raw,plan.target));
        for(const mutate of [r=>{r.package_context.schema_version=1;},r=>{delete r.profile;},r=>{r.source_check=true;},r=>{r.package_context.sources.push(r.package_context.sources[0]);},r=>{r.package_context.manifests=['--untrusted'];}]){
            const bad=structuredClone(raw);mutate(bad);assert.throws(()=>decodeGenericTaskContext(bad,plan.target));
        }
        const own=document(ownPath,await readFile(ownPath,'utf8'));c.send('textDocument/didOpen',{textDocument:{uri:own.uri.toString(),languageId:'sovra',version:1,text:own.text}});
        assert.equal((await resolveTarget(own)).target,plan.target);
        const unregistered=document(notesPath,notes.text);c.send('textDocument/didOpen',{textDocument:{uri:unregistered.uri.toString(),languageId:'sovra',version:1,text:unregistered.text}});
        await assert.rejects(resolveTarget(unregistered),/not registered/);
        await writeFile(manifestPath,manifestText.replace('../helper','../missing'));
        await assert.rejects(resolveTarget(active),/context unavailable/);
        await c.stop();c=undefined;
    }
    if (process.platform === 'linux' && ['x64','ia32','arm64'].includes(process.arch)) {
        const project=join(scratch,'generic diagnostics'),app=join(project,'app'),helper=join(project,'helper');
        await mkdir(app,{recursive:true});await mkdir(helper);
        await writeFile(join(app,'sovra.toml'),'[project]\nname="app"\nentry="main.svr"\n[dependencies.helper]\npath="../helper"\n');
        await writeFile(join(helper,'sovra.toml'),'[project]\nname="helper"\nentry="main.svr"\n[modules]\nvalues="values.svr"\n');
        const entry=join(app,'main.svr'),provider=join(helper,'values.svr');
        const source='use helper::values;fn main(){let x=helper::values::id::<Int>(1);let never_run=1/0;}';
        const good='mod values {export fn id<T>(x:T)->T{return x;}}';
        const bad='// λ😀\r\nmod values {export fn id<T>(x:T)->T{return true;}}';
        await writeFile(entry,source);await writeFile(provider,good);await writeFile(join(helper,'main.svr'),'fn main(){}');
        const entryUri=pathToFileURL(entry).href,providerUri=pathToFileURL(provider).href;
        c=client();
        c.send('initialize',{initializationOptions:{experimentalGenericPackageDiagnostics:true}},60);assert.equal((await c.response(60)).error.code,-32602);
        c.send('initialize',{initializationOptions:{experimentalGenerics:true,experimentalGenericPackageDiagnostics:'true'}},61);assert.equal((await c.response(61)).error.code,-32602);
        c.send('initialize',{rootUri:pathToFileURL(project).href,initializationOptions:{experimentalGenerics:true,experimentalGenericPackageDiagnostics:true}},62);
        const caps=(await c.response(62)).result.capabilities;
        assert.equal(caps.experimental.sovraGenericPackageDiagnostics,true);assert.equal(caps.experimental.sovraPipeline,'generics-package-diagnostics');
        const diagnostics=(uri,version,predicate=()=>true)=>c.wait(m=>m.method==='textDocument/publishDiagnostics'&&m.params.uri===uri&&m.params.version===version&&predicate(m.params.diagnostics),'generic package diagnostics');
        c.send('textDocument/didOpen',{textDocument:{uri:entryUri,languageId:'sovra',version:1,text:source}});
        assert.deepEqual((await diagnostics(entryUri,1)).params.diagnostics,[]);
        c.send('textDocument/didOpen',{textDocument:{uri:providerUri,languageId:'sovra',version:1,text:bad}});
        const failures=(await diagnostics(providerUri,1,d=>d.some(x=>x.code==='E3123'))).params.diagnostics;
        assert.equal(failures.find(x=>x.code==='E3123').range.start.line,1);
        assert.equal(await readFile(provider,'utf8'),good);assert.equal(await readFile(entry,'utf8'),source);
        c.send('textDocument/didChange',{textDocument:{uri:providerUri,version:2},contentChanges:[{text:good}]});
        assert.deepEqual((await diagnostics(providerUri,2,d=>!d.length)).params.diagnostics,[]);
        c.send('textDocument/documentSymbol',{textDocument:{uri:entryUri}},63);assert.deepEqual((await c.response(63)).result.map(x=>x.name),['main']);
        c.send('textDocument/definition',{textDocument:{uri:entryUri},position:{line:0,character:source.indexOf('id::<')}},64);
        const definition=(await c.response(64)).result;assert.equal(definition.uri,providerUri);
        assert.deepEqual(definition.range,{start:{line:0,character:good.indexOf('id<')},end:{line:0,character:good.indexOf('id<')+2}});
        c.send('textDocument/hover',{textDocument:{uri:entryUri},position:{line:0,character:source.indexOf('id::<')}},65);
        assert.ok((await c.response(65)).result.contents.value.includes('fn id<T>(x:T)->T'));
        const member={uri:entryUri,text:source},openMembers=new Map([[entry,member]]);
        const raw=spawnSync(compiler,['check','--experimental-generics','--format','json','--overlays-stdin',app],{input:overlayFrame(81,openMembers),encoding:'utf8',timeout:15000});
        assert.equal(raw.status,0,raw.stdout+raw.stderr);const metadata=JSON.parse(raw.stdout).package_symbols;
        const paths=new Set(metadata.files.map(f=>f.path));assert.ok(await decodePackageNavigation(metadata,paths,openMembers));
        const crossed=structuredClone(metadata);
        const localRef=crossed.local_references[0];
        localRef.definition=crossed.locals.findIndex(x=>x.owner!==localRef.owner || x.file!==localRef.file);
        assert.ok(localRef.definition>=0);
        await assert.rejects(decodePackageNavigation(crossed,paths,openMembers),/ownership/);
        const incomplete=structuredClone(metadata);delete incomplete.local_references;
        await assert.rejects(decodePackageNavigation(incomplete,paths,openMembers),/local tables/);
        const damaged=structuredClone(metadata);damaged.files[0].sha256='0'.repeat(64);
        assert.equal(await decodePackageNavigation(damaged,paths,openMembers),undefined);
        await writeFile(provider,good+' ');assert.equal(await decodePackageNavigation(metadata,paths,openMembers),undefined);await writeFile(provider,good);

        c.send('textDocument/didChange',{textDocument:{uri:providerUri,version:3},contentChanges:[{text:bad}]});
        await diagnostics(providerUri,3,d=>d.some(x=>x.code==='E3123'));
        c.send('textDocument/definition',{textDocument:{uri:entryUri},position:{line:0,character:source.indexOf('id::<')}},66);
        assert.equal((await c.response(66)).result,null);
        c.queued.splice(0);c.send('textDocument/didClose',{textDocument:{uri:providerUri}});
        await diagnostics(entryUri,1,d=>!d.length);
        c.queued.splice(0);await writeFile(provider,bad);c.send('workspace/didChangeWatchedFiles',{changes:[{uri:providerUri,type:2}]});
        await diagnostics(entryUri,1,d=>d.some(x=>x.code==='SVR_LSP'&&x.message.includes('closed source')));
        c.queued.splice(0);await writeFile(provider,good);c.send('workspace/didChangeWatchedFiles',{changes:[{uri:providerUri,type:2}]});
        await diagnostics(entryUri,1,d=>!d.length);
        c.queued.splice(0);c.send('textDocument/didOpen',{textDocument:{uri:providerUri,languageId:'sovra',version:4,text:' '.repeat(512*1024+1)}});
        await diagnostics(entryUri,1,d=>d.some(x=>x.message.includes('rejected')));
        c.queued.splice(0);c.send('textDocument/didClose',{textDocument:{uri:providerUri}});await diagnostics(entryUri,1,d=>!d.length);
        const owner={text:'λ'},members=new Map([[entry,owner]]),entries=new Set([entry]);
        for(const [start,end] of [[1,2],[0,3]]){
            const report={schema_version:1,kind:'project',target:app,success:false,profile:'experimental-generics',overlay_request:{schema_version:1,request_id:3},diagnostics:[{severity:'error',code:'E3123',message:'test',location:{file:entry,start,end}}]};
            assert.throws(()=>packageDiagnostics(report,app,3,entries,members,owner,()=>({line:0,character:0}),'experimental-generics'),/byte boundary/);
            assert.throws(()=>packageDiagnostics(report,app,3,entries,members,owner,()=>({line:0,character:0})),/profile/);
        }
        const typedProvider='// λ😀\r\nmod values {export struct Box<T>{value:T}export enum Choice<T>{One(Box<T>),End}export fn id<T>(x:T)->T{return x;}}';
        const typedEntry='// λ😀\r\nuse helper::values;struct Box<T>{local:T}type Alias<T>=helper :: values :: Box<T>;fn convert<T>(x:Alias<T>)->helper::values::Box<T>{let saved:Alias<T>=x;let qualified:helper :: values :: Box<T>=saved;return qualified;}fn local<T>(x:Box<T>){}fn consume<T>(x:helper::values::Choice<T>){}fn build<T>(x:T){let own=Box::<T>{local:x};let foreign=helper :: values :: Box::<T>{value:x};let choice=helper::values::Choice::<T>::One(foreign);let copied=helper::values::id::<Alias<T>>(foreign);let array=helper::values::id::<[Alias<T>]>([copied]);}fn main(){}';
        c.batch([
            ['textDocument/didChange',{textDocument:{uri:entryUri,version:2},contentChanges:[{text:typedEntry}]}],
            ['textDocument/didOpen',{textDocument:{uri:providerUri,languageId:'sovra',version:5,text:typedProvider}}],
        ]);
        await diagnostics(entryUri,2,d=>!d.length);await diagnostics(providerUri,5,d=>!d.length);
        const line=typedEntry.split('\n')[1],providerLine=typedProvider.split('\n')[1];
        let typeId=200;
        for(const [marker,offset,targetUri,targetLine,name] of [
            ['x:Alias',2,entryUri,line,'Alias'],
            ['Box::<T>{local',0,entryUri,line,'Box'],
            ['helper :: values :: Box::<T>','helper :: values :: '.length,providerUri,providerLine,'Box'],
            ['Choice::<T>',0,providerUri,providerLine,'Choice'],
            ['id::<Alias',6,entryUri,line,'Alias'],
            ['id::<[Alias',7,entryUri,line,'Alias'],
            ['saved:Alias',6,entryUri,line,'Alias'],
            ['qualified:helper :: values :: Box','qualified:helper :: values :: '.length,providerUri,providerLine,'Box'],
            ['helper :: values :: Box','helper :: values :: '.length,providerUri,providerLine,'Box'],
            ['->helper::values::Box','->helper::values::'.length,providerUri,providerLine,'Box'],
            ['x:Box',2,entryUri,line,'Box'],
            ['helper::values::Choice','helper::values::'.length,providerUri,providerLine,'Choice'],
        ]){
            const point={line:1,character:line.indexOf(marker)+offset};
            c.send('textDocument/definition',{textDocument:{uri:entryUri},position:point},typeId);
            const target=(await c.response(typeId++)).result;assert.equal(target.uri,targetUri);
            assert.deepEqual(target.range,{start:{line:1,character:targetLine.indexOf(name+'<')},end:{line:1,character:targetLine.indexOf(name+'<')+name.length}});
            c.send('textDocument/hover',{textDocument:{uri:entryUri},position:point},typeId);
            assert.ok((await c.response(typeId++)).result.contents.value.includes(name+'<T>'));
        }
        c.send('textDocument/documentSymbol',{textDocument:{uri:entryUri}},110);
        const outline=(await c.response(110)).result;
        assert.equal(outline.find(x=>x.name==='Box').kind,23);assert.equal(outline.find(x=>x.name==='Alias').kind,26);
        c.send('textDocument/documentSymbol',{textDocument:{uri:providerUri}},111);
        assert.equal((await c.response(111)).result.find(x=>x.name==='Choice').kind,10);
        const fieldLine='use helper::values;struct Box<T>{value:T}fn inspect<T>(a:Box<T>,b:helper::values::Box<T>){let own=a.value;let other=(b.value);let nested=helper::values::Box::<Box<T>>{value:a};let chain=nested.value.value;}fn main(){}';
        c.send('textDocument/didChange',{textDocument:{uri:entryUri,version:3},contentChanges:[{text:'// λ😀\r\n'+fieldLine}]});
        await diagnostics(entryUri,3,d=>d.length===0);
        let fieldId=300;
        for(const [marker,offset,targetUri,targetLine] of [
            ['a.value',2,entryUri,fieldLine],
            ['b.value',2,providerUri,providerLine],
            ['nested.value.value',7,providerUri,providerLine],
            ['nested.value.value',13,entryUri,fieldLine],
            ['{value:a}',1,providerUri,providerLine],
        ]){
            const point={line:1,character:fieldLine.indexOf(marker)+offset};
            c.send('textDocument/definition',{textDocument:{uri:entryUri},position:point},fieldId);
            const target=(await c.response(fieldId++)).result;
            assert.equal(target.uri,targetUri);
            assert.equal(target.range.start.character,targetLine.indexOf('value:T'));
            c.send('textDocument/hover',{textDocument:{uri:entryUri},position:point},fieldId);
            assert.equal((await c.response(fieldId++)).result.contents.value,'value:T');
        }
        const localLine='fn work(x:Int)->Int{let mut value=x;if true{let value=true;let inner=value;}value=value+1;return value;}fn other(x:Int)->Int{return x;}fn main(){let result=work(1);}';
        c.send('textDocument/didChange',{textDocument:{uri:entryUri,version:4},contentChanges:[{text:'// λ😀\r\n'+localLine}]});
        await diagnostics(entryUri,4,d=>d.length===0);
        let localId=120;
        for(const [marker,offset,declaration,label] of [
            ['value=x',6,'work(x','parameter x: Int'],
            ['inner=value',6,'value=true','let value: Bool'],
            ['value=value+1',0,'value=x','let mut value: Int'],
            ['value=value+1',6,'value=x','let mut value: Int'],
            ['return value',7,'value=x','let mut value: Int'],
            ['return x',7,'other(x','parameter x: Int'],
        ]){
            const point={line:1,character:localLine.indexOf(marker)+offset};
            c.send('textDocument/definition',{textDocument:{uri:entryUri},position:point},localId);
            const target=(await c.response(localId++)).result;
            assert.equal(target.uri,entryUri);
            assert.equal(target.range.start.character,localLine.indexOf(declaration)+(declaration.endsWith('(x')?declaration.length-1:0));
            c.send('textDocument/hover',{textDocument:{uri:entryUri},position:point},localId);
            assert.ok((await c.response(localId++)).result.contents.value.includes(label));
        }
        assert.equal(await readFile(entry,'utf8'),source);assert.equal(await readFile(provider,'utf8'),good);
        await c.stop();c=undefined;
    }
    assert.deepEqual((await readdir(tmpdir())).filter(name => name.startsWith('sovra-lsp-format-') && !before.has(name)), []);
    console.log('LSP generics: explicit/mutually exclusive initialization, unsaved Unicode diagnostics, formatting edits without execution/workspace writes, stale-format retirement, declaration outline/hover, bound call navigation, standalone defaults and explicit generic package tasks, related source saves/dirty-manifest refusal, opt-in unsaved package diagnostics, checked cross-file calls/type annotations/aliases and fingerprint refusal, dependency refresh/close/rejection, static completions, generation replacement and legacy isolation passed.');
} finally { c?.kill(); await rm(scratch, { recursive: true, force: true }); }
