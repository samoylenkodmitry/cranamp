import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const ROOT=path.resolve(import.meta.dirname,'../..');
const OUT=path.join(ROOT,'assets/skins/Catamp Midnight Snack');
export const C={night:'#19182c',dark:'#26253c',shadow:'#38344f',teal:'#35616a',jade:'#86aca4',gold:'#f4bd68',light:'#ffe8b1',pink:'#cd879d',ink:'#322435'};
export async function call(name,args={}){
 const r=await fetch('http://127.0.0.1:18765/mcp',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/call',params:{name,arguments:args}})});
 const b=await r.json();if(!r.ok||b.error||b.result?.isError)throw Error(JSON.stringify(b));
 const t=b.result.content.find(p=>p.type==='text')?.text;try{return JSON.parse(t);}catch{return t;}
}
const op=(op,x,y,extra={})=>({op,x,y,...extra});
const rect=(x,y,width,height,color)=>op('rect',x,y,{width,height,color,fill:true});
const line=(x,y,x2,y2,color)=>op('line',x,y,{x2,y2,color,brush_size:1});
const text=(x,y,text,color,spacing=0)=>op('text',x,y,{text,color,face:'small',spacing});
const stamp=(x,y,rows,palette)=>op('stamp',x,y,{rows,palette});
const draw=(operations,args={})=>call('studio_draw',{operations,states:'all',...args});
const sprite=(origin,operations,states='all')=>draw(operations,{origin,states});
const layer=name=>call('studio_layers',{action:'add',name});
const view=args=>call('studio_canvas',{guides:false,grid:false,clip:null,mask_colors:[],alpha_lock:false,...args});
const STATIC=['main.background','main.title','equalizer.background','equalizer.title','playlist.top.left','playlist.title','playlist.top.right','playlist.bottom.left','playlist.bottom.right'];
const FIXED=['main.options','main.minimize','main.shade','main.close','main.previous','main.play','main.pause','main.stop','main.next','main.eject','main.shuffle','main.repeat','main.eq.toggle','main.playlist.toggle','main.mono','main.stereo','main.position.track','main.volume.track','main.balance.track','equalizer.on','equalizer.auto','equalizer.presets','equalizer.graph','equalizer.close'];
let data;
export async function foundation(){
 await layer('01 · Midnight plum on every control state');
 await draw([rect(0,0,275,377,C.night)],{layers:[...STATIC,...FIXED,'playlist.top.tile','playlist.left.rail','playlist.right.rail','playlist.scroll.thumb','equalizer.band0.track','equalizer.band0.thumb','main.volume.thumb','main.balance.thumb','main.position.thumb']});
 // Source fonts retain their glyph shapes; their ground now matches the joined scene.
 for(const [sheet,w,h]of [['text.bmp',155,18],['numbers.bmp',99,13]]){
  await call('studio_atlas',{sheet});
  const clip=await call('studio_cluster',{rect:[0,0,w,h]});
  const palette=Object.fromEntries(Object.entries(clip.palette).map(([k,c])=>{
   const rgb=[1,3,5].map(i=>parseInt(c.slice(i,i+2),16));return [k,rgb[0]*.3+rgb[1]*.59+rgb[2]*.11>120?C.light:C.night];
  }));
  await draw([stamp(0,0,clip.rows,palette)]);
 }
 await view({active:true,pressed:false,zoom:2,presentation:true,preview_playlist_height:145});
 await call('studio_targets',{auto:true});
}
export async function scene(){
 await layer('02 · Cats inhabit the joined canvas');
 data=(await fs.readFile(path.join(ROOT,'assets/skins/sources/Midnight Snack reference.png'))).toString('base64');
 await draw([op('image',0,0,{data,width:275,height:377})],{layers:[...STATIC,...FIXED]});
 // Fit the tabby's complete lower paw above the shared slider cells.
 await draw([rect(0,116,275,38,C.night),
  op('image',0,116,{data,source_rect:[0,452,1071,215],width:275,height:38})],{layers:[...STATIC,...FIXED]});
 // The two upper kittens occupy real bitmap margins outside the live text.
 await draw([rect(0,8,58,51,C.night),rect(191,14,74,33,C.night),
  op('image',2,2,{data,source_rect:[31,58,183,172],width:40,height:38}),
  op('image',205,0,{data,source_rect:[767,9,299,210],width:34,height:24})],{layers:[...STATIC,...FIXED]});
 // The playlist title has a repeated cell: an unbroken yarn course crosses every copy.
 await sprite('playlist.top.tile',[rect(0,0,25,20,C.night),
  stamp(9,12,['    a    ','   aaa   ','  aabaa  ','   aaa   ','    a    '],{a:C.teal,b:C.jade})]);
 for(const[id,w]of [['playlist.left.rail',12],['playlist.right.rail',20]]){
  await sprite(id,[rect(0,0,w,29,C.night),line(id.includes('left')?3:17,0,id.includes('left')?3:17,28,C.teal),
  stamp(id.includes('left')?1:15,10,['  a  ',' aaa ','aabaa',' aaa ','  a  '],{a:C.teal,b:C.gold})]);
 }
 // Quiet native slider field; shared EQ tracks always carry the same yarn cross section.
 await draw([rect(19,154,239,64,C.night)],{layers:['equalizer.background']});
 // A complete stalking kitten lives in the gap between the preamp and first band.
 // The old large kitten crossed a repeated playlist tile and cannot stay there.
 await draw([rect(0,208,105,44,C.night),
  op('image',36,188,{data,source_rect:[0,800,375,242],width:41,height:27}),
  line(3,208,3,251,C.teal),
  op('curve',3,212,{x2:35,y2:228,control:[7,232],color:C.teal,brush_size:2}),
  op('curve',34,228,{x2:77,y2:242,control:[71,218],color:C.teal,brush_size:1}),
  stamp(8,227,['  a  ',' aaa ','aabaa',' aaa ','  a  '],{a:C.teal,b:C.gold})],{layers:STATIC});
 // The footer cat's complete tail fits beside the time readouts.
 await draw([rect(0,339,228,38,C.night),
  op('image',1,340,{data,source_rect:[60,1240,760,225],width:125,height:37}),
  line(3,339,3,376,C.teal),line(272,339,272,376,C.teal)],{layers:['playlist.bottom.left','playlist.bottom.right']});
 await sprite('equalizer.band0.track',[rect(0,0,14,63,C.night),line(6,1,6,61,C.teal),line(7,1,7,61,C.shadow)]);
 await sprite('main.volume.track',[rect(0,0,68,13,C.night),line(0,6,67,6,C.teal)]);
 await sprite('main.balance.track',[rect(0,0,38,13,C.night),line(0,6,37,6,C.teal)]);
 await sprite('main.position.track',[rect(0,0,248,10,C.night),line(0,6,247,6,C.teal)]);
 // Flat darkness underneath live glyphs, with no engraved readout boxes.
 await draw([rect(43,25,59,16,C.night),rect(110,24,151,19,C.night),rect(110,43,82,12,C.night),
  rect(132,347,95,13,C.night),rect(191,361,35,12,C.night)],{layers:['main.background','playlist.bottom.left','playlist.bottom.right']});
}
const fish=(w,h,held=false)=>{
 const xx=Math.floor((w-9)/2), yy=Math.floor((h-5)/2);
 return [rect(0,0,w,h,C.night),stamp(xx,yy,['  aaa   a',' abbaa aa','abbbcaaaa',' abbaa aa','  aaa   a'],{a:held?C.pink:C.gold,b:C.light,c:C.ink})];
};
export async function details(){
 await layer('03 · Fish charms, paw marks and live lettering');
 await draw([text(80,4,'MIDNIGHT SNACK',C.gold),rect(72,29,2,2,C.light),rect(72,35,2,2,C.light)],{layers:['main.background','main.title']});
 for(const pressed of [false,true]){
  await view({pressed,active:true});
  for(const [id,w,h]of [['main.position.thumb',29,10],['main.volume.thumb',14,11],['main.balance.thumb',14,11],['playlist.scroll.thumb',8,18]]){
   const p=id==='playlist.scroll.thumb'?[rect(0,0,w,h,C.night),stamp(1,5,['  aa  ',' aaaa ','abbbba',' abba ','  aa  '],{a:pressed?C.pink:C.gold,b:C.light})]:fish(w,h,pressed);
   await sprite(id,p,'current');
  }
  await sprite('equalizer.band0.thumb',fish(11,11,pressed),'current');
  // Tiny ink marks sit directly on the fur. Their entire background remains the scene.
  const ink=pressed?C.light:C.ink, y=pressed?13:12;
  for(const [id,rows]of [
   ['main.previous',['x   x','x  xx','x xxx','x  xx','x   x']],
   ['main.play',['x    ','xxx  ','xxxxx','xxx  ','x    ']],
   ['main.pause',['xx xx','xx xx','xx xx','xx xx','xx xx']],
   ['main.stop',['xxxxx','xxxxx','xxxxx','xxxxx','xxxxx']],
   ['main.next',['x   x','xx  x','xxx x','xx  x','x   x']],
   ['main.eject',['  x  ',' xxx ','xxxxx','     ','xxxxx']]
  ])await sprite(id,[stamp(9,y,rows,{x:ink})],'current');
  for(const[id,word]of [['equalizer.presets','PRESETS']]){
   await sprite(id,[text(id==='main.stereo'?1:3,id.startsWith('equalizer')?4:5,word,pressed?C.light:C.gold)],'current');
  }
  for(const[id,rows]of [
   ['main.options',['x x x','xxxxx',' xxx ']],
   ['main.minimize',['     ','     ','xxxxx']],
   ['main.shade',['  x  ',' x x ','x   x']],
   ['main.close',['x   x',' x x ','  x  ',' x x ','x   x']],
   ['equalizer.close',['x   x',' x x ','  x  ',' x x ','x   x']]
  ])await sprite(id,[stamp(2,2,rows,{x:pressed?C.light:C.gold})],'current');
  for(const active of [false,true]){
   await view({active,pressed});
   for(const[id,word]of [['main.mono','MONO'],['main.stereo','STEREO']]){
    await sprite(id,[text(1,5,word,active?C.light:C.jade)],'current');
   }
   for(const[id,word,x,y]of [['main.shuffle','SHUF',12,6],['main.repeat','REP',6,6],['main.eq.toggle','EQ',8,4],['main.playlist.toggle','PL',8,4],['equalizer.on','ON',8,4],['equalizer.auto','AUTO',7,4]]){
    await sprite(id,[text(x,y,word,id.startsWith('equalizer')?(active?C.light:C.jade):(active?C.ink:C.shadow)),
     stamp(2,y+1,[' x ','xxx',' x '],{x:active?(pressed?C.pink:C.gold):C.shadow})],'current');
   }
  }
 }
 await view({active:true,pressed:false});
 await sprite('equalizer.preamp.line',[rect(0,0,113,1,C.ink)]);
 for(let playback=0;playback<3;playback++){
  await view({playback});
  const rows=playback===0?['xxx','xxx','xxx']:playback===1?['x  ','xx ','xxx','xx ','x  ']:['x x','x x','x x','x x','x x'];
  await draw([op('image',2,2,{data,source_rect:[31,58,183,172],width:40,height:38})],{layers:['main.status'],states:'current'});
  await sprite('main.status',[stamp(3,2,rows,{x:C.ink})],'current');
 }
 const p=[text(26,222,'PURR',C.gold),text(190,223,'NO REGRETS',C.gold)];
 await draw(p,{layers:['equalizer.background']});
 const footer=[text(19,349,'+',C.light),text(48,349,'-',C.ink),text(77,349,'OK',C.ink),text(111,349,'...',C.light),text(236,351,'LIST',C.gold)];
 for(const[x,rows]of [[139,['x  x','x xx','xxxx','x xx','x  x']],[148,['x   ','xx  ','xxxx','xx  ','x   ']],[157,['x x','x x','x x','x x','x x']],[166,['xxxx','xxxx','xxxx','xxxx','xxxx']],[175,['x  x','xx x','xxxx','xx x','x  x']],[186,['  x  ',' xxx ','xxxxx','     ','xxxxx']]])footer.push(stamp(x,365,rows,{x:C.gold}));
 footer.push(line(266,369,271,364,C.gold),line(266,373,272,367,C.jade));
 await draw(footer,{layers:['playlist.bottom.left','playlist.bottom.right']});
 await call('studio_options',{playlist_colors:{Normal:'#c9bfd6',Current:C.light,NormalBG:C.night,SelectedBG:'#39314e',MbFG:C.light,MbBG:C.night},
  visualizer_colors:[C.night,C.shadow,C.light,C.gold,C.gold,C.pink,C.pink,C.jade,C.jade,C.teal,C.teal,C.teal,C.shadow,C.shadow,C.night,C.night,C.night,C.night,C.light,C.gold,C.pink,C.jade,C.teal,C.light]});
 await call('studio_cursors',{action:'draw',overwrite:true,style:'paw'});
}
export async function rails(){
 // Every possible 29-pixel tile phase meets the footer at the same cross section.
 for(const[id,w,x]of [['playlist.left.rail',12,3],['playlist.right.rail',20,17]]){
  await sprite(id,[rect(0,0,w,29,C.night),line(x,0,x,28,C.teal)]);
 }
}
export async function polish(){
 await view({active:true,pressed:false,presentation:true});
 await layer('04 · Unbroken yarn, fish moon and native star clusters');
 // Replace the repeated cap's chopped foliage with an open night sky.
 // The fish crosses the EQ/playlist join inside the unique title cell.
 const p=[rect(0,218,275,34,C.night),
  line(3,218,3,251,C.teal),line(272,218,272,251,C.teal),
  op('curve',3,220,{x2:23,y2:244,control:[18,221],color:C.teal,brush_size:1}),
  op('curve',272,220,{x2:253,y2:245,control:[254,225],color:C.teal,brush_size:1}),
  stamp(8,232,['  a   ',' aaa  ','aabaa ',' aaa  ','  a   '],{a:C.teal,b:C.jade}),
  stamp(259,233,['  a   ',' aaa  ','aabaa ',' aaa  ','  a   '],{a:C.teal,b:C.gold}),
  op('ellipse',125,220,{width:25,height:28,color:'#a76642',fill:true}),
  op('ellipse',125,220,{width:24,height:26,color:C.gold,fill:true}),
  op('ellipse',125,220,{width:21,height:21,color:C.light,fill:true}),
  op('ellipse',115,217,{width:24,height:29,color:C.night,fill:true}),
  op('path',0,0,{points:[[146,232],[158,224],[157,232],[160,244],[146,240]],color:'#a76642',fill:true}),
  op('path',0,0,{points:[[146,233],[156,227],[155,233],[157,241],[146,238]],color:C.gold,fill:true}),
  line(149,233,154,230,C.light),
  stamp(141,227,['xx','xx'],{x:C.ink}),
  line(141,242,136,244,C.light),
  stamp(111,233,['  a  ','  a  ','aaaaa','  a  ','  a  '],{a:C.gold}),
  stamp(168,245,['a'],{a:C.jade}),
  text(26,222,'PURR',C.gold),text(190,223,'NO REGRETS',C.gold)];
 await draw(p,{layers:STATIC});
 await sprite('playlist.top.tile',[rect(0,0,25,20,C.night)]);
 await rails();
 await draw([rect(228,339,47,38,C.night),line(272,339,272,376,C.teal),
  op('curve',270,340,{x2:251,y2:373,control:[253,352],color:C.teal,brush_size:1}),
  stamp(238,345,['  aaa   a',' abbaa aa','abbbcaaaa',' abbaa aa','  aaa   a'],{a:C.gold,b:C.light,c:C.ink}),
  text(235,355,'LIST',C.gold),
  line(266,369,271,364,C.gold),line(266,373,272,367,C.jade)
 ],{layers:['playlist.bottom.right']});
 await draw([
  op('curve',3,39,{x2:4,y2:72,control:[-2,57],color:C.teal,brush_size:1}),
  stamp(68,19,[' a ','aaa',' a '],{a:C.gold}),
  stamp(101,16,['a'],{a:C.jade}),stamp(184,19,['a'],{a:C.gold})
 ],{layers:['main.background','main.title']});
}
export async function finish(){
 await view({active:true,pressed:false,playback:0,position:10,volume:20,balance:14,scroll:5,eq:[14,23,18,13,9,6,9,13,17,21,24],zoom:2,presentation:true,preview_playlist_height:145});
 await call('studio_targets',{auto:true});
 console.log(await call('studio_validate'));
 console.log(await call('studio_export',{path:OUT+'.wsz'}));
 console.log(await call('studio_project',{action:'save',path:OUT+'.cstudio'}));
 console.log(await call('studio_screenshot',{presentation:true,panel:'all',path:OUT+'.png'}));
}
export async function build(){
 if((await call('studio_status')).dirty)throw Error('Save current Studio work before replaying.');
 await call('studio_open',{path:path.join(ROOT,'assets/skins/Catamp Moon Garden.wsz')});
 await view({active:true,pressed:false,zoom:2,preview_playlist_height:145});
  await foundation();await scene();await details();await polish();
   const {restoreArtwork,restoreHandles}=await import('./midnight_snack_continuous.mjs');
    await restoreArtwork();await restoreHandles();
   const {restoreJoins}=await import('./midnight_snack_joins.mjs');
    await restoreJoins();
    const {restoreScene}=await import('./midnight_snack_scene.mjs');
    await restoreScene();await finish();
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)await build();
