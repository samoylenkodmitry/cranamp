// Paint through the shared Studio document, never directly into skin atlases.
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {call,C,finish} from './midnight_snack.mjs';
const root=path.resolve(import.meta.dirname,'../..');
const op=(op,x,y,extra={})=>({op,x,y,...extra});
const stamp=(x,y,rows,palette)=>op('stamp',x,y,{rows,palette});
const text=(x,y,text,color)=>op('text',x,y,{text,color,face:'small',spacing:0});
const line=(x,y,x2,y2,color)=>op('line',x,y,{x2,y2,color,brush_size:1});
const rect=(x,y,width,height,color,opacity=255)=>op('rect',x,y,{width,height,color,opacity,fill:true});
const draw=(operations,args={})=>call('studio_draw',{operations,states:'all',label:'Continuous illustrated backdrop',...args});
const sprite=(origin,operations,states='all')=>draw(operations,{origin,states});
const STATIC=['main.background','main.title','equalizer.background','equalizer.title','playlist.top.left','playlist.title','playlist.top.right','playlist.bottom.left','playlist.bottom.right'];
const FIXED=['main.options','main.minimize','main.shade','main.close','main.previous','main.play','main.pause','main.stop','main.next','main.eject','main.shuffle','main.repeat','main.eq.toggle','main.playlist.toggle','main.mono','main.stereo','main.position.track','main.volume.track','main.balance.track','equalizer.on','equalizer.auto','equalizer.presets','equalizer.graph','equalizer.close'];
// The 18-pixel weave period fits a 14-pixel shared track plus its 4-pixel gap.
const fabricPalette={a:'#182735',b:'#1c303c',c:'#223c47',d:'#294751',e:'#35525b',f:'#526868',g:'#786555'};
function cloth(x,y,period=18){
 const xx=((x%period)+period)%period,p=(xx+Math.round(3*Math.sin(y/9))+period*2)%period;
 let c=p<3?'a':p<7?'b':p<12?'c':p<15?'d':'c';
 if((xx+y*2)%17===0&&y%3===0)c=c==='a'?'b':'e';
 if(y%21===8&&(xx===5||xx===7))c='f';
 if(y%21===10&&xx>=5&&xx<=7)c='g';
 return c;
}
function textile(x,y,w,h,phase=0,period=18){
 return stamp(x,y,Array.from({length:h},(_,yy)=>Array.from({length:w},(_,xx)=>cloth(xx+phase,yy,period)).join('')),fabricPalette);
}
function fish(w,h,pressed){
 const rows=w>=25?['       aaaaaaaa          a  ','    aaabbbbbbbbaaa      aa  ','  aabbbbbbbbbbbbbbaa   aaa  ',' abbbbbcbbbbbbbbbbbbaaaaaa  ','aabbbbbbbbbbbbbbbbbbaaaaaaa ',' abbbbbbbbbbbbbbbbbbaaaaaa  ','  aabbbbbbbbbbbbbaaaa   aaa  ','    aaaaaaaaaaaaa       aa  ','                         a  ']:['  aaa   a',' abbaa aa','abbbcaaaa',' abbaa aa','  aaa   a'];
  const shape=w<9?[' aa   a ','abbaa aa','abcbbaaa','abbaa aa',' aa   a ']:rows;
  return [rect(0,0,w,h,'#ff00ff'),stamp(Math.max(0,Math.floor((w-shape[0].length)/2)),Math.floor((h-shape.length)/2),shape,{a:pressed?C.pink:C.gold,b:C.light,c:C.ink})];
}
export async function restoreHandles(){
  await call('studio_canvas',{clip:null,mask_colors:[],alpha_lock:false});
  await call('studio_layers',{action:'add',name:'06 · Fish without lunchboxes'});
  for(const pressed of [false,true]){
    await call('studio_canvas',{active:true,pressed});
    for(const[id,w,h]of [['main.position.thumb',29,10],['main.volume.thumb',14,11],['main.balance.thumb',14,11],['equalizer.band0.thumb',11,11],['playlist.scroll.thumb',8,18]])await sprite(id,fish(w,h,pressed),'current');
  }
  await call('studio_canvas',{active:true,pressed:false});
  await call('studio_targets',{auto:true});
}
// Lift native Studio pixels, undo the temporary crop, then stamp a feathered
// composite. The underlying source and final pixels both pass through MCP.
async function blendImage(image,layers,feather=4){
 const {x,y,width:w,height:h}=image;
 await call('studio_targets',{auto:true});
 const under=await call('studio_cluster',{rect:[x,y,w,h]});
 await draw([image],{layers});
 const over=await call('studio_cluster',{rect:[x,y,w,h]});
 await call('studio_undo');
 const palette={},keys=new Map();
 const rgba=hex=>[1,3,5].map(i=>parseInt(hex.slice(i,i+2),16));
 const rows=Array.from({length:h},(_,yy)=>Array.from({length:w},(_,xx)=>{
  const a=rgba(under.palette[under.rows[yy][xx]]),b=rgba(over.palette[over.rows[yy][xx]]);
  const distance=Math.min(xx,yy,w-1-xx,h-1-yy);
  const alpha=Math.min(1,distance/feather);
  const hex='#'+a.map((c,i)=>Math.round(c+(b[i]-c)*alpha).toString(16).padStart(2,'0')).join('');
  if(!keys.has(hex)){const key=String.fromCharCode(0xe000+keys.size);keys.set(hex,key);palette[key]=hex;}
  return keys.get(hex);
 }).join(''));
 await draw([stamp(x,y,rows,palette)],{layers});
}
export async function restoreArtwork(){
 const data=(await fs.readFile(path.join(root,'assets/skins/sources/Midnight Snack continuous.png'))).toString('base64');
 const original=(await fs.readFile(path.join(root,'assets/skins/sources/Midnight Snack reference.png'))).toString('base64');
 const image=(x,y,width,height,source_rect)=>op('image',x,y,{data,width,height,...(source_rect?{source_rect}:{})});
 await call('studio_canvas',{active:true,pressed:false,preview_playlist_height:145,clip:null,mask_colors:[],alpha_lock:false});
 await call('studio_layers',{action:'add',name:'05 · No holes in the cat attic'});
 await draw([image(0,0,275,377)],{layers:[...STATIC,...FIXED]});
 await draw([image(0,116,275,38,[0,452,1070,215])],{layers:[...STATIC,...FIXED]});
 await draw([image(0,8,58,51,[208,48,222,198])],{layers:['main.background','main.title','main.status']});
 await draw([op('image',2,2,{data:original,source_rect:[31,58,183,172],width:40,height:38})],{layers:['main.background','main.title','main.status','main.options']});
 await draw([rect(43,24,59,18,C.night,95),rect(110,25,152,13,C.night,100),rect(109,39,85,14,C.night,90)],{layers:['main.background']});
 await draw([textile(19,154,239,64,19-78)],{layers:['equalizer.background']});
 await draw([textile(21,154,14,63)],{layers:['equalizer.background']});
 await sprite('equalizer.band0.track',[textile(0,0,14,63),line(6,0,6,62,'#48616a')]);
 await blendImage(image(36,156,41,35,[485,685,289,248]),['equalizer.background'],3);
 await blendImage(image(36,192,41,27,[0,800,375,247]),['equalizer.background'],3);
 await draw([image(0,154,19,64,[0,645,75,140])],{layers:['equalizer.background']});
 await draw([textile(0,218,275,34,0,25)],{layers:['equalizer.background',...STATIC.filter(x=>x.startsWith('playlist.top')||x==='playlist.title')]});
 await sprite('playlist.top.tile',[textile(0,0,25,20,0,25)]);
 await blendImage(image(110,218,64,34,[400,875,270,146]),['equalizer.background','playlist.title'],7);
 await draw([image(127,339,101,38,[410,1080,410,154])],{layers:['playlist.bottom.left','playlist.bottom.right']});
 await draw([textile(0,339,127,38,0,25)],{layers:['playlist.bottom.left','playlist.bottom.right']});
 await draw([op('image',1,340,{data:original,source_rect:[60,1240,760,225],width:125,height:37})],{layers:['playlist.bottom.left','playlist.bottom.right']});
 await draw([rect(132,347,95,13,C.night,110),rect(193,363,34,10,C.night,110)],{layers:['playlist.bottom.right']});
 const railPalette={a:'#192432',b:'#263e49',c:'#36525a',d:'#23303f'};
 for(const[id,w,row]of [['playlist.left.rail',12,'aaabcdcbaaaa'],['playlist.right.rail',20,'aaaaaaddddaaaaabcdcb']])await sprite(id,[stamp(0,0,Array(29).fill(row.padEnd(w,'a')),railPalette)]);
 for(const pressed of [false,true]){
  await call('studio_canvas',{pressed,active:true});
  for(const[id,w,h]of [['main.position.thumb',29,10],['main.volume.thumb',14,11],['main.balance.thumb',14,11],['equalizer.band0.thumb',11,11],['playlist.scroll.thumb',8,18]])await sprite(id,fish(w,h,pressed),'current');
  const ink=pressed?C.light:C.ink;
  for(const[id,rows]of [
   ['main.previous',['x   x','x  xx','x xxx','x  xx','x   x']],['main.play',['x    ','xxx  ','xxxxx','xxx  ','x    ']],['main.pause',['xx xx','xx xx','xx xx','xx xx','xx xx']],['main.stop',['xxxxx','xxxxx','xxxxx','xxxxx','xxxxx']],['main.next',['x   x','xx  x','xxx x','xx  x','x   x']],['main.eject',['  x  ',' xxx ','xxxxx','     ','xxxxx']]
  ])await sprite(id,[stamp(9,pressed?13:12,rows,{x:ink})],'current');
  for(const[id,rows]of [['main.options',['x x x','xxxxx',' xxx ']],['main.minimize',['     ','     ','xxxxx']],['main.shade',['  x  ',' x x ','x   x']],['main.close',['x   x',' x x ','  x  ',' x x ','x   x']],['equalizer.close',['x   x',' x x ','  x  ',' x x ','x   x']]])await sprite(id,[stamp(2,2,rows,{x:pressed?C.light:C.gold})],'current');
  await sprite('equalizer.presets',[text(3,4,'PRESETS',pressed?C.light:C.gold)],'current');
  for(const active of [false,true]){
   await call('studio_canvas',{active,pressed});
   for(const[id,word]of [['main.mono','MONO'],['main.stereo','STEREO']])await sprite(id,[text(1,5,word,active?C.light:C.jade)],'current');
   for(const[id,word,x,y]of [['main.shuffle','SHUF',12,6],['main.repeat','REP',6,6],['main.eq.toggle','EQ',8,4],['main.playlist.toggle','PL',8,4],['equalizer.on','ON',8,4],['equalizer.auto','AUTO',7,4]])await sprite(id,[text(x,y,word,id.startsWith('equalizer')?(active?C.light:C.jade):(active?C.ink:C.shadow))],'current');
  }
 }
 for(let playback=0;playback<3;playback++){
  await call('studio_canvas',{playback});
  const rows=playback===0?['xxx','xxx','xxx']:playback===1?['x  ','xx ','xxx','xx ','x  ']:['x x','x x','x x','x x','x x'];
  await sprite('main.status',[stamp(3,2,rows,{x:C.ink})],'current');
 }
 await call('studio_canvas',{active:true,pressed:false,playback:0});
 await draw([text(80,4,'MIDNIGHT SNACK',C.gold),rect(72,29,2,2,C.light),rect(72,35,2,2,C.light)],{layers:['main.background','main.title']});
 await draw([text(23,222,'PURR',C.light),text(190,223,'NO REGRETS',C.light)],{layers:['equalizer.background']});
 const footer=[text(19,349,'+',C.light),text(48,349,'-',C.ink),text(77,349,'OK',C.ink),text(111,349,'...',C.light),text(236,354,'LIST',C.ink)];
 for(const[x,rows]of [[139,['x  x','x xx','xxxx','x xx','x  x']],[148,['x   ','xx  ','xxxx','xx  ','x   ']],[157,['x x','x x','x x','x x','x x']],[166,['xxxx','xxxx','xxxx','xxxx','xxxx']],[175,['x  x','xx x','xxxx','xx x','x  x']],[186,['  x  ',' xxx ','xxxxx','     ','xxxxx']]])footer.push(stamp(x,365,rows,{x:C.gold}));
 await draw(footer,{layers:['playlist.bottom.left','playlist.bottom.right']});
 await call('studio_targets',{auto:true});
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
 if((await call('studio_status')).dirty)throw Error('Save current Studio work before repainting.');
 await restoreArtwork();await finish();
}
