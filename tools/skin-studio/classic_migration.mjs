/** One-time migration of Cranamp's bundled artwork. Every edit uses Skin Studio's pen.
 * Static key holes inherit the continuous underlying illustration. Moving cells get
 * explicitly designed track cross-sections; they never capture a local background.
 * Run against a Studio build with studio_font. Originals are saved before any edit.
 */
import fs from 'node:fs/promises';
import path from 'node:path';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
const run = promisify(execFile);
const root = path.resolve(import.meta.dirname, '../..');
const endpoint = 'http://127.0.0.1:18765/mcp';
let serial = 1;
export async function studio(name, args = {}) {
  const response = await fetch(endpoint, {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({jsonrpc:'2.0',id:serial++,method:'tools/call',params:{name,arguments:args}})});
  const json = await response.json();
  if (json.error || json.result?.isError) throw Error(JSON.stringify(json.error || json.result));
  const text = json.result.content.filter(c=>c.type==='text').map(c=>c.text).join('\n');
  try { return JSON.parse(text); } catch { return text; }
}
const palettes = {
  'Cardboard':['#fff1bd','#604624'], 'Cat Scan':['#d7e6d7','#162b25'],
  'Catnip':['#674558','#f3ead0'], 'Feral Night':['#efd481','#171323'],
  'Freefall':['#93cddd','#0c1a36'], 'Midnight Snack':['#fae5b1','#1a2034','#263d48','#34505a','#758d82'],
  'Moon Garden':['#ecedde','#142335'], 'Moonlit':['#674558','#f3ead0'],
  'Purr Chaos Font Fixed':['#fff0c4','#263d35','#b5cbae','#abc2a5','#7d9b7d'],
  'Purr Chaos Polished':['#fff0c4','#263d35','#b5cbae','#abc2a5','#7d9b7d'],
  'Purr Chaos':['#fff0c4','#263d35','#b5cbae','#abc2a5','#7d9b7d'],
  'Purrmission Atelier':['#fff0c4','#263d35','#b5cbae','#abc2a5','#7d9b7d'],
  'Purrmission Pixel Edition':['#fff0c4','#263d35','#b5cbae','#abc2a5','#7d9b7d'],
  'Purrmission':['#315e4b','#bad2bf','#bad2bf','#aec6b4','#779984'],
  'Salvage':['#cee5c6','#073344'], 'Sampler':['#ece3c4','#2c384e'],
  'Seance':['#efd481','#191322'], 'Silverplay':['#caf1f4','#12394b'],
};
const rgb = color => [1,3,5].map(i=>parseInt(color.slice(i,i+2),16));
const hex = c => '#'+c.slice(0,3).map(v=>v.toString(16).padStart(2,'0')).join('');
const key = c => c && c[0]===255 && c[1]===0 && c[2]===255;
function bmp(bytes) {
  if(bytes.toString('ascii',0,2)!=='BM') throw Error('Expected BMP');
  const width=bytes.readInt32LE(18), signedHeight=bytes.readInt32LE(22), height=Math.abs(signedHeight), bits=bytes.readUInt16LE(28), offset=bytes.readUInt32LE(10), stride=Math.ceil(width*bits/32)*4;
  if(![8,24,32].includes(bits)||bytes.readUInt32LE(30)!==0) throw Error('Unsupported migration input BMP encoding');
  const pixels=new Array(width*height);
  for(let y=0;y<height;y++)for(let x=0;x<width;x++){
    let i=offset+(signedHeight>0?height-1-y:y)*stride+x*(bits/8);
    if(bits===8)i=14+bytes.readUInt32LE(14)+bytes[i]*4;
    pixels[y*width+x]=[bytes[i+2],bytes[i+1],bytes[i]];
  }
  return {width,height,pixels,at(x,y){return this.pixels[y*this.width+x];}};
}
export async function migrateSkin(stem) {
  if(!palettes[stem])throw Error('Skin needs an explicitly reviewed palette: '+stem);
  const name='Catamp '+stem;
  const dir=path.join(root,'assets/skins');
  const backup=path.join(root,'target/classic-migration/originals');
  const reportDir=path.join(root,'target/classic-migration/after');
  await fs.mkdir(backup,{recursive:true}); await fs.mkdir(reportDir,{recursive:true});
  for(const ext of ['wsz','cstudio','png']){
    const destination=path.join(backup,name+'.'+ext);
    try {await fs.access(destination);} catch {
      try {await fs.copyFile(path.join(dir,name+'.'+ext),destination);} catch(e){if(e.code!=='ENOENT')throw e;}
    }
  }
  const source=path.join(backup,name+'.wsz');
  const old=(await run('/usr/bin/unzip',['-Z1',source])).stdout.trim().split('\n');
  const sheets=new Map();
  for(const file of old.filter(n=>n.toLowerCase().endsWith('.bmp'))){
    const data=(await run('/usr/bin/unzip',['-p',source,file],{encoding:'buffer',maxBuffer:10_000_000})).stdout;
    sheets.set(path.basename(file).toLowerCase(),bmp(data));
  }
  let project=false;
  try {await fs.access(path.join(backup,name+'.cstudio'));project=true;} catch {}
  if(project)await studio('studio_project',{action:'open',path:path.join(backup,name+'.cstudio'),discard:true});
  else await studio('studio_open',{path:source,discard:true});
  await studio('studio_state',{panel:'canvas',layers:[],active:true,pressed:false,states:'all',guides:false,presentation:true,zoom:2,alpha_lock:false,mask_colors:[],clip:null});
  const rectangles=(await studio('studio_rectangles',{})).rectangles.filter(r=>r.active&&!r.hit&&!r.runtime&&r.source);
  const targets=(await studio('studio_targets',{variants:true})).sprites;
  const [ink,bg,track=bg,edge=bg,line=ink]=palettes[stem];
  const fallback=rgb(bg), writes=new Map();
  const queue=(sheet,x,y,color)=>{
    const im=sheets.get(sheet); if(!im||x<0||y<0||x>=im.width||y>=im.height)return;
    if(!writes.has(sheet))writes.set(sheet,new Map());
    writes.get(sheet).set(y*im.width+x,typeof color==='string'?rgb(color):color);
  };
  const moving = id => id.endsWith('.thumb')||id.endsWith('.track')||id.includes('.digit')||id.endsWith('.preamp.line');
  function under(index,x,y){
    let color=fallback;
    for(let j=0;j<index;j++){
      const r=rectangles[j]; if(moving(r.id))continue;
      const [dx,dy,w,h]=r.rect; if(x<dx||y<dy||x>=dx+w||y>=dy+h)continue;
      const [sx,sy,sw,sh]=r.source;
      const p=sheets.get(r.sheet)?.at(sx+Math.floor((x-dx)*sw/w),sy+Math.floor((y-dy)*sh/h));
      if(p&&!key(p))color=p;
    }
    return color;
  }
  const originalKeys=[...sheets.values()].reduce((n,im)=>n+im.pixels.filter(key).length,0);
  if(originalKeys){
    // Only fill holes of fixed cells from the continuous background. Preserve all ink.
    const done=new Set();
    for(let index=0;index<rectangles.length;index++){
      const r=rectangles[index]; if(moving(r.id))continue;
      const target=targets.find(t=>t.id===r.id); if(!target)continue;
      const im=sheets.get(r.sheet); if(!im)continue;
      for(const cell of target.variants){
        const token=r.sheet+':'+cell.join(',');if(done.has(token))continue;done.add(token);
        for(let y=0;y<cell[3];y++)for(let x=0;x<cell[2];x++){
          if(key(im.at(cell[0]+x,cell[1]+y)))queue(r.sheet,cell[0]+x,cell[1]+y,under(index,r.rect[0]+Math.floor(x*r.rect[2]/cell[2]),r.rect[1]+Math.floor(y*r.rect[3]/cell[3])));
        }
      }
    }
    // Shared rails are constant along the direction of movement. Paw/fish shapes
    // retain their original pixels; their surrounds match these exact cross-sections.
    const vertical=x=>x===6?line:(x===0||x===13?edge:track);
    const horizontal=y=>y===6?line:(y===0||y===12?edge:track);
    const seek=y=>y===5?line:(y===0||y===9?edge:track);
    for(const id of ['equalizer.band0.track','main.volume.track','main.balance.track','main.position.track']){
      const t=targets.find(t=>t.id===id);if(!t)continue;
      const color=id.startsWith('equalizer')?(x,y)=>vertical(x):id.includes('position')?(x,y)=>seek(y):(x,y)=>horizontal(y);
      for(const c of t.variants)for(let y=0;y<c[3];y++)for(let x=0;x<c[2];x++)queue(t.sheet,c[0]+x,c[1]+y,color(x,y));
    }
    for(const id of ['equalizer.band0.thumb','main.volume.thumb','main.balance.thumb','main.position.thumb','playlist.scroll.thumb']){
      const t=targets.find(t=>t.id===id);if(!t)continue;const im=sheets.get(t.sheet);
      for(const c of t.variants)for(let y=0;y<c[3];y++)for(let x=0;x<c[2];x++){
        if(!key(im.at(c[0]+x,c[1]+y)))continue;
        const color=id.startsWith('equalizer')?vertical(x+1):id.includes('position')?seek(y):id.startsWith('playlist')?(x===3?line:bg):horizontal(y+1);
        queue(t.sheet,c[0]+x,c[1]+y,color);
      }
    }
    // The scrolling backdrop must match at every 29-pixel repeat phase.
    for(let y=42;y<71;y++)for(let x=36;x<44;x++)queue('pledit.bmp',x,y,x===39?line:bg);
    // Digits use a shared opaque face. Blank slot 10 must truly be blank.
    for(const sheet of ['numbers.bmp','nums_ex.bmp']){
      const im=sheets.get(sheet);if(!im)continue;
      for(let y=0;y<13;y++)for(let x=0;x<im.width;x++)if(key(im.at(x,y))||(x>=90&&x<99))queue(sheet,x,y,bg);
    }
    for(let y=25;y<40;y++)for(let x=46;x<100;x++){
      // Retain the separator artwork between digit cells; fill only old key pixels.
      if(key(sheets.get('main.bmp').at(x,y)))queue('main.bmp',x,y,bg);
    }
    // Preamp is a genuine moving one-pixel line, never a copied picture slice.
    for(let x=0;x<113;x++)queue('eqmain.bmp',x,314,edge);
    for(let y=294;y<313;y++)queue('eqmain.bmp',115,y,ink);
    // Unused key pixels are harmless but removing them avoids resurrecting the old
    // convention if a previously unused state is mapped by a future player.
    for(const [sheet,im] of sheets)for(let i=0;i<im.pixels.length;i++)if(key(im.pixels[i])&&!writes.get(sheet)?.has(i))queue(sheet,i%im.width,Math.floor(i/im.width),bg);
  }
  await studio('studio_layers',{action:'add',name:'Classic Winamp compatibility · opaque controls and 5x6 font'});
  let operations=0;
  for(const [sheet,changes] of writes){
    await studio('studio_state',{panel:'atlas',sheet,layers:[],clip:null,mask_colors:[],alpha_lock:false,opacity:255,mirror_x:false,mirror_y:false});
    const im=sheets.get(sheet), indices=[...changes.keys()].sort((a,b)=>a-b), ops=[];
    for(let i=0;i<indices.length;i++){
      const start=indices[i], c=hex(changes.get(start));let end=start;
      while(i+1<indices.length&&indices[i+1]===end+1&&Math.floor((end+1)/im.width)===Math.floor(start/im.width)&&hex(changes.get(indices[i+1]))===c){end++;i++;}
      ops.push({op:'rect',x:start%im.width,y:Math.floor(start/im.width),width:end-start+1,height:1,fill:true,color:c});
    }
    for(let i=0;i<ops.length;i+=8000){const report=await studio('studio_draw',{operations:ops.slice(i,i+8000),label:'Classic opaque '+sheet});if(report.unmapped_pixels>0)throw Error('Unmapped migration ink');}
    operations+=ops.length;
  }
  await studio('studio_state',{panel:'canvas',layers:[]});
  if(stem.startsWith('Purr Chaos')) {
    // Clear the interrupted old yarn only on its background, then redraw it in
    // the continuous margin below the moving controls, with Studio's native pen.
    await studio('studio_draw',{layers:['equalizer.background'],states:'all',label:'Reconnect the loose yarn below the paw strings',operations:[
      {op:'rect',x:78,y:154,width:176,height:63,fill:true,color:track},
      {op:'curve',x:93,y:221,x2:148,y2:222,control:[124,240],brush_size:1,color:'#748e89'},
      {op:'curve',x:148,y:222,x2:199,y2:221,control:[174,207],brush_size:1,color:'#748e89'},
      {op:'curve',x:199,y:221,x2:248,y2:223,control:[222,238],brush_size:1,color:'#748e89'},
      {op:'curve',x:110,y:220,x2:166,y2:221,control:[138,232],brush_size:1,color:'#b78277'},
      {op:'curve',x:166,y:221,x2:217,y2:220,control:[191,212],brush_size:1,color:'#b78277'}
    ]});
  }
  await studio('studio_font',{ink,background:bg});
  const options=await studio('studio_options',{});
  const colors=options.visualizer_colors;
  if(colors[0].toLowerCase()==='#ff00ff'){
    colors[0]=bg; if(colors[1].toLowerCase()==='#ff00ff')colors[1]=bg;
    await studio('studio_options',{visualizer_colors:colors});
  }
  await studio('studio_state',{panel:'canvas',layers:[],presentation:true,zoom:2,active:true,pressed:false,volume:20,balance:14,position:11,scroll:11,eq:[14,14,14,14,14,14,14,14,14,14,14]});
  const validation=await studio('studio_validate',{});
  if(!validation.exportable)throw Error(JSON.stringify(validation));
  await studio('studio_export',{path:path.join(dir,name+'.wsz')});
  await studio('studio_screenshot',{panel:'all',presentation:true,path:path.join(dir,name+'.png')});
  await fs.copyFile(path.join(dir,name+'.png'),path.join(reportDir,name+'.png'));
  const report={name,originalKeys,operations,validation};
  await fs.writeFile(path.join(reportDir,name+'.json'),JSON.stringify(report,null,2));
  return {name,originalKeys,operations,exportable:validation.exportable};
}
export const bundledNames = Object.keys(palettes);
