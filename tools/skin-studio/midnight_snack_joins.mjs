// One canvas coordinate system. All artwork edits go through the shared Studio.
import {call,C,finish} from './midnight_snack.mjs';
import {pathToFileURL} from 'node:url';
import fs from 'node:fs/promises';
const palette=['#182735','#1c303c','#223c47','#294751','#35525b','#526868','#786555'];
const rgba=s=>[1,3,5].map(i=>parseInt(s.slice(i,i+2),16));
const hex=a=>'#'+a.map(v=>Math.round(v).toString(16).padStart(2,'0')).join('');
function fabric(x,y) {
  const xx=((x%25)+25)%25, yy=y-154, p=(xx+Math.round(3*Math.sin(yy/9))+50)%25;
  let i=p<4?0:p<9?1:p<17?2:p<21?3:2;
  if((xx+yy*2)%17===0&&yy%3===0)i=i===0?1:4;
  if(yy%21===8&&(xx===7||xx===9))i=5;
  if(yy%21===10&&xx>=7&&xx<=9)i=6;
  return palette[i];
}
function nativeStamp(x,y,w,h,colorAt) {
  const colors={}, symbols=new Map();
  const rows=Array.from({length:h},(_,dy)=>Array.from({length:w},(_,dx)=>{
    const color=colorAt(x+dx,y+dy); if(!color)return ' ';
    if(!symbols.has(color)){const c=String.fromCodePoint(0xe000+symbols.size);symbols.set(color,c);colors[c]=color;}
    return symbols.get(color);
  }).join(''));
  return {op:'stamp',x,y,rows,palette:colors};
}
const reviews=[];
const draw=async(operations,args={})=>{
  const out=await call('studio_draw',{operations,layers:[],states:'all',label:'07 · One cloth, one canvas',...args});
  reviews.push({targets:args.origin??args.layers??'Auto',continuity:out.continuity,overwrites:out.overwrites});
  return out;
};
const pixel=(cluster,x,y)=>cluster.palette[cluster.rows[y][x]];
export async function restoreJoins(){
  reviews.length=0;
  await call('studio_canvas',{active:true,pressed:false,clip:null,mask_colors:[],alpha_lock:false,preview_playlist_height:145});
  await call('studio_layers',{action:'add',name:'07 · One cloth, one canvas'});
  // The omitted 113x1 overlay was an old stripe across the cat's paw.
  await draw([{op:'rect',x:0,y:0,width:113,height:1,fill:true,color:'#ff00ff'}],{origin:'equalizer.preamp.line'});
  // Shared slider cells carry silhouettes only. A single backing canvas now owns
  // the cloth instead of eleven copied backgrounds with a different repeat pitch.
  await draw([{op:'rect',x:0,y:0,width:14,height:63,fill:true,color:'#ff00ff'}],{origin:'equalizer.band0.track'});
  await call('studio_targets',{auto:true});
  const before=await call('studio_cluster',{rect:[0,0,275,377]});
  before.rows=before.rows.map(row=>[...row]); // One Unicode code point per native pixel.
  const fabricColors=new Set(palette);
  const cloth=nativeStamp(0,154,275,98,(x,y)=>{
    if(y<218&&(x<19||x>=258))return null;
    return fabricColors.has(pixel(before,x,y).slice(0,7).toLowerCase())?fabric(x,y):null;
  });
  // One stamp spanning the panel join, preserving cats, lettering and the star.
  const staticTargets=['equalizer.background','playlist.top.left','playlist.title','playlist.top.tile','playlist.top.right'];
  await draw([cloth],{layers:staticTargets,require_continuity:true});
  // Remove the ruler-straight footer splice with a short native transition,
  // retaining the purple cat and the gold transport glyphs as foreground ink.
  const footer=nativeStamp(127,339,18,38,(x,y)=>{
    const current=rgba(pixel(before,x,y));
    const gold=current[0]>170&&current[1]>100&&current[2]<current[0]*0.8;
    if(gold)return null;
    const a=rgba(fabric(x,y-185)), b=current;
    const t=(x-127)/17, eased=t*t*(3-2*t);
    return hex(a.map((v,i)=>v*(1-eased)+b[i]*eased));
  });
  await draw([footer],{require_continuity:true});
  // These are complete joined-canvas strokes across EQ, title, repeated rails
  // and footer. A constant section also joins correctly at every playlist height.
  const cords=[];
  for(const x of [3,271]){
    cords.push({op:'line',x,y:223,x2:x,y2:376,brush_size:3,color:'#182735'});
    cords.push({op:'line',x,y:223,x2:x,y2:376,brush_size:1,color:'#526868'});
  }
  await draw(cords,{require_continuity:true});
  await call('studio_targets',{auto:true});
  await fs.mkdir('target/continuity-review',{recursive:true});
  await fs.writeFile('target/continuity-review/strokes.json',JSON.stringify(reviews,null,2));
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
  if((await call('studio_status')).dirty)throw Error('Save current Studio work before repairing joins.');
  await restoreJoins();await finish();
}
