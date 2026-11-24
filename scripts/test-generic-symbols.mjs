import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { decodeEvidence } from '../toolchain/ide/language-server/symbols.mjs';
const directory=await mkdtemp(join(tmpdir(),'sovra-generic-symbols-'));
const compiler=resolve(process.env.SOVRA_TEST_COMPILER || 'target/debug/svr');
const source='// λ😀\r\ntype Same<T>=T;\nstruct Box<T>{value:T}\nenum Choice<T>{One(T),End}\nfn pick<T>(x:std::Option<T>)->std::Option<T>{return std::Option::<T>::Some(x?);}\nfn main(){let x=pick::<Int>(std::Option::<Int>::Some(7));}';
const file=join(directory,'main.svr');
const cli=args=>{const result=spawnSync(compiler,args,{encoding:'utf8',timeout:20000,maxBuffer:2*1024*1024});if(result.error)throw result.error;return result;};
try{
    await writeFile(file,source);
    const args=['check','--experimental-generics','--format','json'];
    const ordinary=cli([...args,file]);assert.equal(ordinary.status,0,ordinary.stderr);assert.equal(JSON.parse(ordinary.stdout).symbols,undefined);
    const result=cli([...args,'--symbols',file]);assert.equal(result.status,0,result.stderr);const report=JSON.parse(result.stdout);assert.equal(report.success,true);const bytes=Buffer.from(source);const evidence=decodeEvidence(report.symbols,bytes);assert.deepEqual(evidence.declarations.map(d=>d.name),['Same','Box','Choice','pick','main']);assert.deepEqual(evidence.declarations.map(d=>d.kind),[26,23,10,12,12]);assert.equal(evidence.references.length,1);assert.equal(evidence.references[0].definition,3);assert.equal(bytes.subarray(evidence.references[0].start,evidence.references[0].end).toString(),'pick');assert.deepEqual(evidence.locals.map(x=>x.name),['x','x']);assert.deepEqual(evidence.locals.map(x=>x.signature),['parameter x: std::Option<T>','let x: std::Option<Int>']);assert.equal(evidence.localReferences.length,1);assert.equal(evidence.localReferences[0].definition,0);
    for(const d of evidence.declarations){assert.equal(bytes.subarray(d.name_start,d.name_end).toString(),d.name);assert.equal(bytes.subarray(d.start,d.end).toString().includes(d.name),true);}
    assert.equal(evidence.declarations[3].signature,'fn pick<T>(x:std::Option<T>)->std::Option<T>');assert.equal(evidence.declarations[0].selectionRange.start.line,1);
    for(const args of [['check','--experimental-generics','--symbols',file],['check','--experimental-generics','--format','json','--symbols','--symbols',file],['run','--experimental-generics','--symbols',file],['check','--experimental-generics','--format','json','--symbols',directory]])assert.equal(cli(args).status,2);
    const scoped='fn helper<T>(x:T)->T{let y=x;return y;}fn main(){let x=1;if true{let x=x+1;let y=(x);}let z=x;let mut a:[Int]=[z];a[0]=z;}';
    await writeFile(file,scoped);const scopedResult=cli([...args,'--symbols',file]);assert.equal(scopedResult.status,0,scopedResult.stdout+scopedResult.stderr);
    const facts=decodeEvidence(JSON.parse(scopedResult.stdout).symbols,Buffer.from(scoped));
    for(const [token,delta,target] of [['let y=x',6,0],['return y',7,1],['let x=x+1',6,2],['let y=(x)',7,3],['let z=x',6,2],['a[0]=z',0,6]]){
        const at=scoped.indexOf(token)+delta;const ref=facts.localReferences.find(r=>r.start===at);assert.ok(ref,token);assert.equal(ref.definition,target,token);
    }
    const typed='// λ😀\r\nstruct Box<T>{value:T}type Alias<T>=Box<T>;fn unused<T>(x:Alias<T>)->[Box<T>]{return [x];}fn main(){}';
    await writeFile(file,typed);const typedResult=cli([...args,'--symbols',file]);assert.equal(typedResult.status,0,typedResult.stdout+typedResult.stderr);
    const typedFacts=decodeEvidence(JSON.parse(typedResult.stdout).symbols,Buffer.from(typed));
    assert.deepEqual(typedFacts.references.map(r=>[Buffer.from(typed).subarray(r.start,r.end).toString(),r.definition]),[['Alias',1],['Box',0]]);
    await writeFile(file,'fn main(){missing();}');const bad=cli([...args,'--symbols',file]);assert.equal(bad.status,1);assert.equal(JSON.parse(bad.stdout).symbols,undefined);
    await writeFile(file,'fn main('+' '.repeat(4200)+'){}');
    const unavailable=cli([...args,'--symbols',file]);assert.equal(unavailable.status,0);const optional=JSON.parse(unavailable.stdout);assert.equal(optional.success,true);assert.equal(optional.symbols.unavailable,true);assert.equal(decodeEvidence(optional.symbols,Buffer.from('fn main('+' '.repeat(4200)+'){}')),undefined);
    console.log('Generic symbols: opt-in complete declaration envelopes, checked original UTF-16/name ranges, typed kinds/headers, bound call identity, strict flag combinations and failed-check suppression passed.');
}finally{await rm(directory,{recursive:true,force:true});}
