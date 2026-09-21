// Correct the annotated composition through native Studio drawing and palette calls.
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {call,finish} from './midnight_snack.mjs';
const root=path.resolve(import.meta.dirname,'../..');
const rgb=s=>[1,3,5].map(i=>parseInt(s.slice(i,i+2),16));
const hex=a=>'#'+a.map(v=>Math.round(v).toString(16).padStart(2,'0')).join('');
const smooth=t=>{t=Math.max(0,Math.min(1,t));return t*t*(3-2*t);};
const mix=(a,b,t)=>hex(rgb(a).map((v,i)=>v*(1-t)+rgb(b)[i]*t));
function shadow(x,y){
 const phase=(x%25)+Math.round(3*Math.sin((y-154)/9));
 const fold=0.7+0.3*Math.sin(phase*Math.PI*2/25);
 const falloff=Math.max(0,Math.min(1,(252-y)/38));
 return mix('#19182c','#284954',fold*falloff*falloff);
}
const unpack=c=>({...c,rows:c.rows.map(r=>[...r])});
const colorAt=(c,x,y)=>c.palette[c.rows[y][x]];
function stamp(x,y,w,h,pixel){
 const palette={},keys=new Map();
 const rows=Array.from({length:h},(_,yy)=>Array.from({length:w},(_,xx)=>{
  const color=pixel(xx,yy);
  if(!keys.has(color)){const key=String.fromCodePoint(0xe000+keys.size);keys.set(color,key);palette[key]=color;}
  return keys.get(color);
 }).join(''));
 return {op:'stamp',x,y,rows,palette};
}
export async function restoreScene(){
 await call('studio_canvas',{active:true,pressed:false,preview_playlist_height:145,clip:null,mask_colors:[],alpha_lock:false});
 await call('studio_targets',{auto:true});
 const before=unpack(await call('studio_cluster',{rect:[0,0,275,377]}));
 await call('studio_layers',{action:'add',name:'08 · The quilt was never a rectangle'});
 const data=(await fs.readFile(path.join(root,'assets/skins/sources/Midnight Snack continuous.png'))).toString('base64');
 // Continue the same source at y667, immediately after the source at the
 // cat's lower bedding edge. No independent kitten stickers or side strips.
 await call('studio_targets',{layers:['equalizer.background']});
 await call('studio_draw',{states:'all',layers:['equalizer.background'],operations:[
  {op:'image',x:0,y:154,width:275,height:78,data,source_rect:[0,667,1070,433]}
 ]});
 const scene=unpack(await call('studio_cluster',{rect:[0,154,275,78]}));
 await call('studio_undo');
 const bedding=stamp(0,154,275,78,(x,dy)=>{
  const y=154+dy;
  // The whole illustration settles into a shared shadow field. That field
  // continues through the title and reaches the exact playlist fill at y252.
  const t=smooth((y-218-Math.round(2*Math.sin(x/31)))/10);
  return mix(colorAt(scene,x,dy),shadow(x,y),t);
 });
 const report=await call('studio_draw',{states:'all',layers:['equalizer.background'],label:'Continuous illustrated bedding',operations:[bedding]});
 // Fish handles intentionally float above the painted background. Do not
 // copy those moving sprites into the source while sampling the illustration.
 if(report.continuity?.mismatches>0&&!report.continuity.samples.every(s=>s.covering_sources.some(p=>p.id.endsWith('.thumb'))))throw Error('Unexpected backdrop occlusion: '+JSON.stringify(report.continuity));
 const title=stamp(0,232,275,20,(x,dy)=>shadow(x,232+dy));
 const cap=await call('studio_draw',{states:'all',layers:[],require_continuity:true,label:'One shadow through the playlist title',operations:[title]});
 const footer=stamp(0,339,275,13,(x,dy)=>{
  const depth=8+Math.round(2*Math.sin(x/27)+Math.sin(x/11));
  return mix(colorAt(before,x,338),colorAt(before,x,339+dy),smooth(dy/depth));
 });
 const edge=await call('studio_draw',{states:'all',layers:[],require_continuity:true,label:'Playlist shadow into the sleeping cat',operations:[footer]});
 const cords=[];
 for(const x of [3,271]){
  cords.push({op:'line',x,y:223,x2:x,y2:376,brush_size:3,color:'#182735'});
  cords.push({op:'line',x,y:223,x2:x,y2:376,brush_size:1,color:'#526868'});
 }
 await call('studio_draw',{states:'all',layers:[],require_continuity:true,label:'Continuous outer threads',operations:cords});
 const options=await call('studio_options');
 options.visualizer_colors[0]='#ff00ff';
 await call('studio_options',{visualizer_colors:options.visualizer_colors});
 await call('studio_targets',{auto:true});
 await fs.mkdir(path.join(root,'target/annotated-abrupt-review'),{recursive:true});
 await fs.writeFile(path.join(root,'target/annotated-abrupt-review/draws.json'),JSON.stringify({bedding:report.continuity,title:cap.continuity,footer:edge.continuity},null,2));
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
 if((await call('studio_status')).dirty)throw Error('Save current Studio work before repairing the scene.');
 await restoreScene();await finish();
}
