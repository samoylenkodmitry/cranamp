import fs from 'node:fs/promises';
import path from 'node:path';
import { call } from './moonlit.mjs';
const canvas = args => call('studio_canvas', args);

// Capture every declared correction with an exterior halo. Ownership is evidence,
// not an aesthetic verdict: the generated board deliberately leaves review pending.
const [manifestFile, phase, output='target/skin-region-review']=process.argv.slice(2);
if(!manifestFile||!['before','after'].includes(phase))throw new Error('Usage: node tools/skin-studio/region_review.mjs manifest.json before|after [output]');
const manifest=JSON.parse(await fs.readFile(manifestFile,'utf8'));
const out=path.resolve(output);
await fs.mkdir(out,{recursive:true});
const status=await call('studio_status');
const saved=status.view;
const height=manifest.playlist_height??145;
const padding=manifest.padding??4;
const restore=Object.fromEntries(['active','pressed','position','volume','balance','scroll','eq','playback','preview_playlist_height','zoom','presentation','guides','grid'].map(k=>[k,saved[k]]));
let report={};
try{report=JSON.parse(await fs.readFile(path.join(out,'review.json'),'utf8'));}catch(e){if(e.code!=='ENOENT')throw e;}
report.name=manifest.name;
report[phase]={review_status:'pending visual inspection',project:status.path,revision:status.revision,regions:[]};
const escape=s=>String(s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
try{
  await canvas({preview_playlist_height:height,active:true,pressed:false,zoom:2,position:10,volume:20,balance:14,scroll:5,eq:[14,23,18,13,9,6,9,13,17,21,24],playback:0});
  for(const region of manifest.regions){
    if(!/^[a-z0-9_-]+$/.test(region.id))throw new Error('Region ids must be safe file stems');
    const [x,y,w,h]=region.rect;
    const a=Math.max(0,x-padding),b=Math.max(0,y-padding);
    const crop=[a,b,Math.min(275,x+w+padding)-a,Math.min(232+height,y+h+padding)-b];
    const prefix=phase+'-'+region.id;
    const study=await call('studio_study',{rect:region.rect,padding,states:true,zoom:4,path:path.join(out,prefix+'-states.png')});
    const coverage=study.study.paintability;
    await call('studio_canvas',{active:true,pressed:false,presentation:false,crop,magnify:1,path:path.join(out,prefix+'-native.png')});
    await call('studio_canvas',{crop,magnify:4,path:path.join(out,prefix+'-editor.png')});
    const images=[];
    for(const active of [true,false])for(const pressed of [false,true]){
      await canvas({active,pressed,presentation:true});
      const file=prefix+'-gpu-'+Number(active)+Number(pressed)+'.png';
      // GPU crops already include the player's zoom. Keep a full-width review
      // under the endpoint's 2048-pixel edge limit instead of failing at 4x.
      const magnify=Math.max(1,Math.min(4,Math.floor(2048/(Math.max(crop[2],crop[3])*2))));
      await call('studio_screenshot',{presentation:true,panel:'all',crop,magnify,path:path.join(out,file)});
      images.push({active,pressed,file});
    }
    report[phase].regions.push({...region,crop,coverage,images,state_board:prefix+'-states.png'});
    console.log(phase,region.id,coverage.counts);
  }
}finally{
  await canvas(restore);
  if(saved.panel==='atlas')await call('studio_atlas',{sheet:saved.sheet});
}
await fs.writeFile(path.join(out,'review.json'),JSON.stringify(report,null,2));
const cards=manifest.regions.map(r=>{
  const cells=['before','after'].map(p=>{
    const region=report[p]?.regions.find(n=>n.id===r.id);
    if(!region)return '<section><h3>'+p+'</h3><p>Not captured</p></section>';
    const prefix=p+'-'+r.id;
    return '<section><h3>'+p+'</h3><img class="native" src="'+prefix+'-native.png" alt="Native editor crop"><div class="image editor"><img src="'+prefix+'-editor.png" alt="Editor"></div>'+region.images.map(i=>'<div class="image gpu-'+Number(i.active)+Number(i.pressed)+'" hidden><img src="'+i.file+'" alt="GPU state"></div>').join('')+'<div class="image state-board" hidden><img src="'+region.state_board+'" alt="All four editor states"></div><details><summary>Ownership and runtime coverage</summary><pre>'+escape(JSON.stringify(region.coverage,null,2))+'</pre></details></section>';
  });
  return '<article><h2>'+escape(r.id)+'</h2><p>'+escape(r.expectation)+'</p><small>Native rectangle '+r.rect.join(', ')+'; '+padding+' px exterior halo. Visual review required.</small><div class="pair">'+cells.join('')+'</div></article>';
}).join('');
await fs.writeFile(path.join(out,'index.html'),'<!doctype html><meta charset="utf-8"><title>'+escape(manifest.name)+'</title><style>body{background:#10141d;color:#e6e7ea;font:16px system-ui;margin:24px}header{position:sticky;top:0;background:#10141d;padding:12px;z-index:1}article{padding:20px 0;border-bottom:1px solid #394354}.pair{display:flex;gap:28px;align-items:flex-start}section{min-width:0}img{image-rendering:pixelated;max-width:100%}.native{display:block;margin:12px 0}pre{max-width:600px;overflow:auto;font-size:12px}select{font:inherit;padding:7px}small{color:#aab7c9}h2{margin-bottom:5px}</style><header><h1>'+escape(manifest.name)+'</h1><label>Inspect <select id="view"><option value="editor">Editor</option><option value="state-board">All four editor states</option><option value="gpu-10">GPU active / released</option><option value="gpu-11">GPU active / pressed</option><option value="gpu-00">GPU inactive / released</option><option value="gpu-01">GPU inactive / pressed</option></select></label><p>Read every crop and its outer halo. Counts describe mapping, not visual quality.</p></header>'+cards+'<script>document.querySelector("#view").onchange=e=>document.querySelectorAll(".image").forEach(n=>n.hidden=!n.classList.contains(e.target.value));</script>');
console.log(path.join(out,'index.html'));
