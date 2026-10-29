import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import { connectionUrl } from '../../tools/skin-studio/connection.mjs';

export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
export const review = path.join(root, 'target/aqua-cat-studio');
export const output = path.join(root, 'assets/skins/Catamp Aqua Mischief');
let requestId = 1;
const C = { paper:'#eaf4f8', silver:'#dfecef', edge:'#5d8195', deep:'#173d50', ink:'#254d64', glass:'#143b4d', aqua:'#52c7ec', pink:'#d983a1', white:'#ffffff', eq:'#e4eef2' };
const rect=(x,y,width,height,color,extra={})=>({op:'rect',x,y,width,height,color,fill:true,...extra});
const ellipse=(x,y,width,height,color,extra={})=>({op:'ellipse',x,y,width,height,color,fill:true,...extra});
const line=(x,y,x2,y2,color,brush_size=1)=>({op:'line',x,y,x2,y2,color,brush_size});
const shape=(points,color,extra={})=>({op:'path',x:0,y:0,points,color,fill:true,...extra});
const curve=(points,color,brush_size=1)=>({op:'path',x:0,y:0,points,color,fill:false,brush_size});
const text=(x,y,value,color=C.ink,extra={})=>({op:'text',x,y,text:value,color,face:'small',spacing:1,...extra});
function ramp(a,b,n=20){return Array.from({length:n},(_,i)=>'#'+[1,3,5].map(k=>Math.round(parseInt(a.slice(k,k+2),16)*(1-i/(n-1))+parseInt(b.slice(k,k+2),16)*i/(n-1)).toString(16).padStart(2,'0')).join(''));}
function pill(x,y,w,h,pressed=false,tint='#168fd4'){
  return [ellipse(x,y+1,w,h,C.edge),ellipse(x,y,w,h,C.deep),
    ellipse(x+1,y+1,w-2,h-2,tint,{ramp:ramp(pressed?'#1576b5':'#d1f7ff',pressed?'#5bc5ef':tint),ramp_axis:[x,y,x,y+h]}),
    ellipse(x+3,y+2,w-6,Math.max(2,Math.floor(h/2)-2),C.white,{opacity:pressed?45:155}),
    line(x+Math.floor(w*.3),y+h-3,x+Math.floor(w*.7),y+h-3,pressed?'#4d9dc4':'#85ebff')];
}
function rounded(x,y,w,h,r,color,extra={}){return shape([[x+r,y],[x+w-r,y],[x+w,y,x+w,y+r],[x+w,y+h-r],[x+w,y+h,x+w-r,y+h],[x+r,y+h],[x,y+h,x,y+h-r],[x,y+r],[x,y,x+r,y]],color,extra);}
function fish(x,y,w=14,h=8,color='#f4ac7e'){
  return [shape([[x,y+h/2],[x-4,y],[x-4,y+h]],color),ellipse(x,y,w,h,color),
    line(x+4,y+1,x+7,y+h-2,'#ffe6b6'),ellipse(x+w-4,y+2,2,2,C.deep)];
}
function icon(name,w,h,ink=C.deep){
  const x=Math.floor(w/2),y=Math.floor(h/2);
  const tri=(cx,reverse=false)=>shape([[cx+(reverse?3:-3),y-4],[cx+(reverse?-3:3),y],[cx+(reverse?3:-3),y+4]],ink);
  if(name==='play')return [tri(x)];
  if(name==='previous'||name==='next')return [tri(x,name==='previous'),rect(x+(name==='previous'?-6:5),y-4,2,9,ink)];
  if(name==='pause')return [rect(x-4,y-4,3,9,ink),rect(x+2,y-4,3,9,ink)];
  if(name==='stop')return [rect(x-3,y-3,7,7,ink)];
  if(name==='eject')return [shape([[x-5,y+1],[x,y-5],[x+5,y+1]],ink),rect(x-5,y+3,11,2,ink)];
  return [];
}
async function draw(label,operations,args={}){
  const report=await call('studio_draw',{label,operations,states:'all',...args});
  await fs.writeFile(path.join(review,'reports',label.replace(/[^a-z0-9]+/gi,'-')+'.json'),JSON.stringify(report,null,2));
  if(report.pixels_written===0 && report.continuity?.checked===0 && report.clipped_pixels>0) throw new Error(label+': every stroke was clipped');
  console.log(label);
  return report;
}
async function layer(name){await call('studio_layers',{action:'add',name});}
async function at(id,label,ops,extra={}){
  // Ramp axes use canvas coordinates; origin translates shape coordinates only.
  if(ops.some(op=>op.ramp_axis)) {
    const {rectangles}=await call('studio_rectangles');
    const {rect:[x,y]}=rectangles.find(r=>r.id===id && !r.hit);
    ops=ops.map(op=>op.ramp_axis?{...op,ramp_axis:op.ramp_axis.map((n,i)=>n+(i%2?y:x))}:op);
  }
  return draw(label,ops,{origin:id,layers:[id],...extra});
}

async function paint(){
  const status=await call('studio_status');
  if(status.dirty || status.undo) throw new Error('Open a new blank project before running the recipe.');
  await call('studio_canvas',{surface:'skin',active:true,pressed:false,grid:false,guides:false});
  await call('studio_probe',{path:path.join(review,'pixel-coverage.json')});
  await layer('01 Silver shell and opaque source cells');
  for(const [sheet,w,h] of status.sheets){
    await call('studio_atlas',{sheet});
    await draw('Opaque '+sheet,[rect(0,0,w,h,C.silver)]);
  }
  await call('studio_atlas',{});
  await call('studio_canvas',{surface:'skin'});
  await draw('Silver joined shell',[rect(0,0,275,377,C.silver)],{layers:['main.background','main.title','equalizer.background','equalizer.title','playlist.top.left','playlist.top.tile','playlist.title','playlist.top.right','playlist.left.rail','playlist.right.rail','playlist.bottom.left','playlist.bottom.right']});
  await at('equalizer.background','Equalizer porcelain',[rect(0,0,275,116,C.eq),line(1,0,1,115,'#ffffff'),line(273,0,273,115,C.edge),line(0,115,274,115,C.edge)]);
  await layer('02 The icon cat wears the player');
  const cat=[
    shape([[7,44],[16,10],[50,30]],C.edge),shape([[10,40],[18,15],[45,31]],'#b7ced8'),shape([[17,34],[20,21],[36,32]],C.pink),
    shape([[224,29],[259,10],[270,47]],C.edge),shape([[232,31],[257,15],[266,42]],'#b7ced8'),shape([[241,31],[255,21],[260,35]],C.pink),
    ellipse(3,17,268,94,C.edge),ellipse(5,17,264,91,C.silver),
    curve([[9,65],[10,94,50,106],[174,115,256,102],[269,85,266,64]],'#b6d0db',2),
    rounded(125,15,4,10,2,'#8da9b6'),rounded(134,15,5,12,2,'#8da9b6'),rounded(145,15,4,10,2,'#8da9b6'),
    rounded(19,21,87,42,9,C.edge),rounded(20,21,85,40,9,'#69cadd'),rounded(24,25,77,34,5,C.glass),
    rounded(106,21,162,34,8,C.edge),rounded(107,21,160,32,8,'#69cadd'),rounded(111,25,152,25,4,C.glass),
    rect(103,29,7,6,'#69cadd'),line(29,23,95,23,'#c0f7ff'),line(118,23,252,23,'#c0f7ff'),
    shape([[55,63],[64,63],[60,68]],C.pink),curve([[51,67],[53,73,60,68],[65,73,70,67]],'#648b9d',1),
    line(3,61,21,65,'#84a4b4'),line(2,67,20,68,'#84a4b4'),line(257,59,272,55,'#84a4b4'),line(257,64,273,63,'#84a4b4'),
    ...fish(190,37,12,5),ellipse(204,37,2,2,'#63c2d7'),ellipse(207,34,2,2,'#92e1ee'),
    ...fish(49,47,20,10),text(111,41,'',C.aqua),text(133,43,'KBPS','#9acbd5'),text(174,43,'KHZ','#9acbd5'),
  ];
  await draw('A silver cat with oversized equalizer glasses',cat,{layers:['main.background','main.title']});
  await layer('03 Aqua title bars');
  for(const id of ['main.title','equalizer.title']){
    await at(id,id+' silver chrome',[rect(0,0,275,14,C.silver,{ramp:ramp('#ffffff','#bdcdd8',14),ramp_axis:[0,0,0,14]}),line(0,0,274,0,'#ffffff'),line(0,13,274,13,C.edge)]);
    await at(id,id+' caption',[text(30,4,id==='main.title'?'AQUA MISCHIEF':'PURR EQUALIZER',C.ink)]);
  }
  for(const [id,color,symbol] of [['main.options','#70c9e1',''],['main.minimize','#f5c84a','-'],['main.shade','#7acb68','+'],['main.close','#ec877a','x'],['equalizer.close','#ec877a','x']]){
    await at(id,id+' traffic light',[rect(0,0,9,9,'#d8e5eb'),...pill(1,1,7,7,false,color),...(symbol==='-'?[line(3,4,5,4,'#6a5a36')]:symbol==='+'?[line(3,4,5,4,'#42683c'),line(4,3,4,5,'#42683c')]:symbol==='x'?[line(3,3,5,5,'#813f3c'),line(5,3,3,5,'#813f3c')]:[ellipse(3,3,3,3,'#e8fcff')])]);
  }
  await layer('04 Glossy playback controls');
  for(const [name,w,h] of [['previous',23,18],['play',23,18],['pause',23,18],['stop',23,18],['next',22,18],['eject',22,16]]){
    for(const pressed of [false,true]){
      await call('studio_canvas',{pressed});
      await at('main.'+name,name+(pressed?' pressed':' released'),[rect(0,0,w,h,C.silver),...pill(1,1,w-2,h-3,pressed),...icon(name,w,h-1)],{states:'current'});
    }
  }
  await call('studio_canvas',{pressed:false});
  for(const [id,w,h,label] of [['main.shuffle',47,15,'SHUF'],['main.repeat',28,15,'REP'],['main.eq.toggle',23,12,'EQ'],['main.playlist.toggle',23,12,'PL'],['equalizer.on',25,12,'ON'],['equalizer.auto',33,12,'AUTO'],['equalizer.presets',44,12,'PRESETS']]){
    for(const active of [false,true])for(const pressed of [false,true]){
      await call('studio_canvas',{active,pressed});
      await at(id,id+(active?' on':' off')+(pressed?' pressed':''),[rect(0,0,w,h,id.startsWith('equalizer')?C.eq:C.silver),...pill(1,0,w-2,h-1,pressed,active?'#168fd4':'#90aaba'),text(1,Math.floor((h-5)/2),label,active?'#08364f':'#395363',{align:'center',width:w-2,spacing:0})],{states:'current'});
    }
  }
  await call('studio_canvas',{active:true,pressed:false});
  for(const [id,w] of [['main.mono',27],['main.stereo',29]]){
    for(const active of [false,true]){
      await call('studio_canvas',{active});
      await at(id,id+(active?' lit':' dim'),[rect(0,0,w,12,C.glass),text(0,5,id==='main.mono'?'MONO':'STEREO',active?'#a1e4ee':'#608692',{width:w,align:'center',spacing:0}),ellipse(2,1,3,2,active?'#78ebbb':'#466778')],{states:'current'});
    }
  }
  await call('studio_canvas',{active:true,pressed:false});
  for(const [id,w,h] of [['main.position.track',248,10],['main.volume.track',68,13],['main.balance.track',38,13]]){
    await at(id,id+' continuous rail',[rect(0,0,w,h,C.silver),rounded(0,Math.floor(h/2)-2,w,5,2,'#6a8b9e'),rect(2,Math.floor(h/2)-1,w-4,3,'#9fd9ec'),line(2,Math.floor(h/2)-1,w-3,Math.floor(h/2)-1,'#efffff')]);
  }
  for(const [id,w,h,railY] of [['main.position.thumb',29,10,5],['main.volume.thumb',14,11,5],['main.balance.thumb',14,11,5]]){
    for(const pressed of [false,true]){
      await call('studio_canvas',{pressed});
      await at(id,id+(pressed?' pressed':' released'),[rect(0,0,w,h,C.silver),rect(0,railY-2,w,5,'#6a8b9e'),rect(0,railY-1,w,3,'#9fd9ec'),line(0,railY-1,w-1,railY-1,'#efffff'),...pill(1,0,w-2,h-1,pressed),line(Math.floor(w/2)-2,3,Math.floor(w/2)-2,h-4,'#e3fbff'),line(Math.floor(w/2)+1,3,Math.floor(w/2)+1,h-4,'#e3fbff')],{states:'current'});
    }
  }
  await layer('05 The fish thief and its tail');
  await call('studio_canvas',{active:true,pressed:false});
  const thief=[
    curve([[63,207],[78,207,75,192],[73,185,70,189]],'#7898aa',7),curve([[63,207],[76,206,73,192],[72,189,70,190]],'#d8e7ec',4),
    ellipse(42,182,27,31,'#7f9dac'),ellipse(44,182,23,30,'#dbe8ed'),ellipse(49,187,13,23,'#f5f9fa'),
    shape([[39,171],[40,155],[51,164]],'#7f9dac'),shape([[60,163],[72,154],[71,175]],'#7f9dac'),
    shape([[42,165],[42,159],[48,164]],C.pink),shape([[64,164],[69,159],[69,169]],C.pink),
    ellipse(37,162,37,27,'#9cb5c1'),ellipse(38,161,35,25,'#e1edf0'),
    rounded(49,163,2,4,1,'#7e9eae'),rounded(54,162,2,5,1,'#7e9eae'),rounded(59,163,2,4,1,'#7e9eae'),
    rounded(40,169,14,10,3,'#55c6dd'),rounded(57,169,14,10,3,'#55c6dd'),rect(53,172,5,2,'#55c6dd'),
    rounded(42,171,10,6,2,C.glass),rounded(59,171,10,6,2,C.glass),
    rect(44,174,2,2,'#6de0cf'),rect(47,172,2,4,'#83f1f1'),rect(61,173,2,3,'#83f1f1'),rect(64,172,2,4,'#6de0cf'),
    shape([[53,179],[58,179],[56,182]],C.pink),curve([[51,182],[53,185,56,182],[59,185,61,182]],'#6d8b9b'),
    ellipse(55,184,3,4,'#e993b0'),line(37,179,44,180,'#7d9ba8'),line(68,180,75,178,'#7d9ba8'),
    curve([[65,193],[72,183,72,157]],'#91acb9',6),curve([[65,192],[70,182,71,157]],'#dbe8ed',3),ellipse(68,154,7,7,'#dbe8ed'),
    curve([[71,154],[77,145,63,146]],'#718593'),line(63,146,63,151,'#718593'),...fish(53,148,10,5),
    ellipse(40,209,13,8,'#a8c1cd'),ellipse(55,210,15,7,'#a8c1cd'),line(44,212,44,215,'#7899aa'),line(48,212,48,215,'#7899aa'),line(60,212,60,215,'#7899aa'),line(64,212,64,215,'#7899aa'),
    text(41,222,'MEOW',C.ink,{spacing:1}),
  ];
  await draw('The sunglasses cat stole the fish',thief,{layers:['equalizer.background']});
  await layer('06 Aqua EQ tracks and graph');
  await at('equalizer.graph','Clear glass graph',[rect(0,0,113,19,C.eq),rounded(0,0,113,19,3,C.edge),rounded(1,1,111,17,3,'#f6fbfc'),line(3,3,109,3,'#ffffff'),line(3,16,109,16,'#d0e3eb')]);
  await at('equalizer.preamp.line','Graph reference line',[rect(0,0,113,1,'#bed6e0')]);
  await call('studio_eq',{action:'edit_frame',frame:14,scope:'all'});
  await at('equalizer.band0.track','Shared EQ porcelain and rail',[rect(0,0,14,63,C.eq),rounded(5,0,4,63,2,'#8aa9ba'),rect(6,1,2,61,'#b5e3f1'),line(6,2,6,60,'#faffff')]);
  await call('studio_eq',{action:'copy_frame_to_all'});
  for(const pressed of [false,true]){
    await call('studio_eq',{action:'edit_handle',pressed});
    await at('equalizer.band0.thumb','Aqua EQ pearl '+pressed,[rect(0,0,11,11,C.eq),rect(4,0,4,11,'#8aa9ba'),rect(5,0,2,11,'#b5e3f1'),line(5,0,5,10,'#faffff'),...pill(0,0,11,10,pressed),line(3,6,7,6,'#d7f7ff')],{states:'current'});
  }
  await call('studio_canvas',{surface:'skin',active:true,pressed:false});
  const labels=['PRE','60','170','310','600','1K','3K','6K','12K','14K','16K'];
  const xs=[21,78,96,114,132,150,168,186,204,222,240];
  await draw('Equalizer frequency labels',labels.map((s,i)=>text(xs[i]-1,222,s,C.ink,{width:16,align:'center',spacing:0})),{layers:['equalizer.background']});
  await layer('07 Playlist and fish-list footer');
  for(const [id,w] of [['playlist.top.left',25],['playlist.top.tile',25],['playlist.title',100],['playlist.top.right',25]]){
    await at(id,id+' joined chrome',[rect(0,0,w,20,C.paper),rect(0,0,w,14,C.silver,{ramp:ramp('#ffffff','#bdcdd8',14),ramp_axis:[0,0,0,14]}),line(0,0,w-1,0,C.white),line(0,13,w-1,13,'#a2bdcb')]);
  }
  await at('playlist.title','The fish list caption',[text(0,4,'THE FISH LIST',C.ink,{width:100,align:'center'})]);
  await at('playlist.top.left','A paw on the playlist title',[ellipse(8,6,7,5,'#6eabbf'),ellipse(7,2,3,3,'#6eabbf'),ellipse(11,1,3,3,'#6eabbf'),ellipse(15,3,3,3,'#6eabbf')]);
  await at('playlist.top.right','Playlist window controls',[...pill(14,3,8,8,false,'#ec877a'),line(16,5,19,8,'#813f3c'),line(19,5,16,8,'#813f3c'),line(3,7,8,7,C.edge)]);
  await at('playlist.left.rail','Left rail repeats without a seam',[rect(0,0,12,29,C.paper),rect(0,0,2,29,'#9bb9c8'),line(2,0,2,28,C.white)]);
  await at('playlist.right.rail','Right rail repeats without a seam',[rect(0,0,20,29,C.paper),rect(5,0,8,29,'#c4dce7'),line(6,0,6,28,'#faffff'),rect(18,0,2,29,'#9bb9c8'),line(17,0,17,28,C.white)]);
  for(const pressed of [false,true]){
    await call('studio_canvas',{pressed});
    await at('playlist.scroll.thumb','Playlist glass thumb '+pressed,[rect(0,0,8,18,'#c4dce7'),line(1,0,1,17,'#faffff'),...pill(0,0,8,18,pressed),line(2,8,5,8,'#dcfaff'),line(2,10,5,10,'#8bd9f1')],{states:'current'});
  }
  await call('studio_canvas',{pressed:false});
  const foot=[rect(0,339,275,38,C.paper),rect(0,344,275,33,C.silver,{ramp:ramp('#edf7fa','#bdcfda',33),ramp_axis:[0,344,0,377]}),line(0,376,274,376,C.edge)];
  for(const [x,w,label] of [[10,27,'+'],[39,27,'-'],[69,27,'OK'],[99,25,'...'],[228,28,'LIST']])foot.push(...pill(x,347,w,16),text(x,352,label,C.deep,{width:w,align:'center',spacing:0}));
  foot.push(text(10,368,'DO NOT FEED THE DJ',C.ink,{spacing:0}),rounded(130,346,96,13,3,C.edge),rect(132,348,92,9,C.glass),rect(191,362,33,10,C.glass));
  for(const [i,name] of ['previous','play','pause','stop','next','eject'].entries()){
    const x=139+i*9; const mini=icon(name,10,10,'#3c7d9e').map(o=>({...o,x:(o.x||0)+x,y:(o.y||0)+362,...(o.x2!==undefined?{x2:o.x2+x,y2:o.y2+362}:{})}));
    foot.push(...mini);
  }
  foot.push(line(263,370,270,363,'#6f96aa'),line(263,374,273,364,'#6f96aa'));
  await draw('Aqua playlist footer',foot,{layers:['playlist.bottom.left','playlist.bottom.right']});
  await call('studio_atlas',{sheet:'pledit.bmp'});
  await draw('Resize footer repeat cells',[rect(179,0,101,38,C.paper),rect(179,5,101,33,C.silver,{ramp:ramp('#edf7fa','#bdcfda',33),ramp_axis:[0,5,0,38]}),rect(207,10,71,18,C.glass),line(179,37,279,37,C.edge)]);
  await call('studio_atlas',{sheet:'numbers.bmp'});
  const digits=[rect(0,0,99,13,C.glass)];
  const segments=['abcedf','bc','abged','abgcd','fgbc','afgcd','afgecd','abc','abcdefg','abfgcd'];
  const sg={a:[2,2,6,2],b:[7,3,7,5],c:[7,7,7,9],d:[2,10,6,10],e:[1,7,1,9],f:[1,3,1,5],g:[2,6,6,6]};
  for(let d=0;d<10;d++)for(const k of segments[d]){const [x,y,x2,y2]=sg[k];digits.push(line(d*9+x,y,d*9+x2,y2,'#8be7f2'));}
  digits.push(line(92,6,96,6,'#8be7f2')); await draw('Turquoise digital timer',digits);
  await call('studio_atlas',{sheet:'playpaus.bmp'});
  await draw('Playback status lights',[rect(0,0,42,9,C.glass),shape([[1,1],[7,4],[1,7]],'#91ecce'),rect(10,1,2,7,'#a3d7e2'),rect(14,1,2,7,'#a3d7e2'),rect(20,2,5,5,'#a3d7e2')]);
  await call('studio_atlas',{});
  await call('studio_font',{ink:'#b6f0f7',background:C.glass});
  await call('studio_options',{playlist_colors:{Normal:C.ink,Current:'#076e9c',NormalBG:C.paper,SelectedBG:'#c6e4f4',MbFG:C.ink,MbBG:C.paper},visualizer_colors:[C.glass,C.glass,...ramp('#a1ffff','#409fbb',16),'#90e8f1','#80dde7','#70ccd9','#63bcca','#50acba','#d2ffff']});
  await call('studio_canvas',{surface:'shade',active:true,pressed:false});
  await at('title','Rolled-up Aqua player',[rect(0,0,275,14,C.silver,{ramp:ramp('#ffffff','#bdcdd8',14),ramp_axis:[0,0,0,14]}),text(20,4,'AQUA CAT',C.ink),line(0,13,274,13,C.edge)]);
  await at('unshade','Unroll Aqua player',[rect(0,0,9,9,'#d8e5eb'),...pill(1,1,7,7,false,'#7acb68'),line(3,4,5,4,'#42683c'),line(4,3,4,5,'#42683c')]);
  await at('position.track','Shade seek rail',[rect(0,0,17,7,'#d8e5eb'),rect(0,2,17,3,'#9fd9ec'),line(0,2,16,2,'#faffff')]);
  await at('position.thumb','Shade seek pearl',[rect(0,0,3,7,C.edge),rect(1,1,1,5,'#73d9f1')]);
  await call('studio_canvas',{surface:'skin',active:true,pressed:false,volume:20,balance:14,position:9,scroll:0,eq:Array(11).fill(14)});
  await call('studio_cursors',{action:'draw',style:'paw',overwrite:true});
  await finishDetails();
  await call('studio_canvas',{surface:'skin'});
  await call('studio_project',{action:'save',path:output+'.cstudio'});
  execFileSync('python3',[path.join(root,'test/skin-studio/seed-optional-atlas.py'),output+'.cstudio',review+'/with-optional-canvas.cstudio']);
  await call('studio_project',{action:'open',path:review+'/with-optional-canvas.cstudio'});
  await paintExtras();
  await call('studio_project',{action:'save',path:output+'.cstudio'});
  await call('studio_canvas',{path:review+'/first-composition.png',zoom:3});
  await call('studio_screenshot',{path:review+'/first-gpu.png',presentation:true,panel:'all'});
  console.log(JSON.stringify(await call('studio_validate')));
}
async function finishDetails(){
  await call('studio_canvas',{surface:'skin',active:true,pressed:false});
  await layer('08 Small controls and EQ curve');
  const ink='#3c7d9e';
  await draw('Readable mini transports',[
    rect(138,362,58,13,C.silver,{ramp:ramp('#edf7fa','#bdcfda',33),ramp_axis:[0,344,0,377]}),rect(191,362,33,10,C.glass),
    line(139,365,139,370,ink),shape([[145,365],[141,368],[145,371]],ink),
    shape([[149,365],[154,368],[149,371]],ink),
    rect(158,365,2,6,ink),rect(162,365,2,6,ink),rect(167,365,5,6,ink),
    shape([[176,365],[180,368],[176,371]],ink),line(182,365,182,370,ink),
    shape([[185,368],[188,364],[190,368]],ink),line(185,370,190,370,ink),
  ],{layers:['playlist.bottom.right']});
  await call('studio_atlas',{sheet:'eqmain.bmp'});
  await draw('Aqua EQ response ink',[rect(115,294,1,19,'#16729d',{ramp:ramp('#0d6390','#4496ab',19),ramp_axis:[115,294,115,312]})],{layers:[]});
  await call('studio_atlas',{});
  await call('studio_canvas',{surface:'skin'});
}
async function paintExtras(){
  await layer('09 Collapsed windows and popup controls');
  await call('studio_atlas',{sheet:'eq_ex.bmp'});
  const eq=[rect(0,0,275,116,C.silver)];
  for(const y of [0,15]) {
    eq.push(rect(0,y,275,14,C.silver,{ramp:ramp('#ffffff','#bdcdd8',14),ramp_axis:[0,y,0,y+14]}),text(12,y+4,'PURR',C.ink),line(0,y+13,274,y+13,C.edge));
    for(const [x,w] of [[61,97],[164,43]])eq.push(rect(x,y+4,w,7,'#d8e5eb'),rect(x,y+6,w,3,'#9fd9ec'),line(x,y+6,x+w-1,y+6,'#faffff'));
    eq.push(...fish(225,y+4,10,5),...pill(254,y+3,9,9,false,'#7acb68'),...pill(264,y+3,9,9,false,'#ec877a'),line(256,y+7,260,y+7,'#42683c'),line(258,y+5,258,y+9,'#42683c'),line(266,y+5,270,y+9,'#813f3c'),line(270,y+5,266,y+9,'#813f3c'));
  }
  for(const x of [1,4,7,11,14,17])eq.push(rect(x,30,3,7,C.edge),rect(x+1,31,1,5,'#73d9f1'));
  for(const y of [38,47])for(const x of [1,11])eq.push(rect(x,y,9,9,'#d8e5eb'),...pill(x,y,9,9,y===47,x===1?'#7acb68':'#ec877a'),...(x===1?[line(x+2,y+4,x+6,y+4,'#42683c'),line(x+4,y+2,x+4,y+6,'#42683c')]:[line(x+2,y+2,x+6,y+6,'#813f3c'),line(x+6,y+2,x+2,y+6,'#813f3c')]));
  await draw('Aqua collapsed equalizer',eq,{layers:[]});
  await call('studio_atlas',{sheet:'pledit.bmp'});
  const pl=[];
  for(const [x,y,w] of [[72,42,25],[72,57,25],[99,42,50],[99,57,50]]){
    pl.push(rect(x,y,w,14,C.silver,{ramp:ramp('#ffffff','#bdcdd8',14),ramp_axis:[x,y,x,y+14]}),line(x,y+13,x+w-1,y+13,C.edge));
    if(w===50)pl.push(...pill(x+30,y+3,9,9,false,'#7acb68'),...pill(x+40,y+3,9,9,false,'#ec877a'),line(x+32,y+7,x+36,y+7,'#42683c'),line(x+34,y+5,x+34,y+9,'#42683c'),line(x+42,y+5,x+46,y+9,'#813f3c'),line(x+46,y+5,x+42,y+9,'#813f3c'));
  }
  for(const [x,labels] of [[0,['URL','DIR','FILE']],[54,['ALL','CROP','SEL','MISC']],[104,['INV','NONE','ALL']],[154,['SORT','INFO','OPTS']],[204,['NEW','SAVE','LOAD']]]){
    for(const pressed of [false,true])for(const [i,label] of labels.entries()){
      const px=x+(pressed?23:0),py=111+i*19;
      pl.push(rect(px,py,22,18,C.silver),...pill(px+1,py+1,20,15,pressed),text(px+1,py+6,label,C.deep,{width:20,align:'center',spacing:0}));
    }
  }
  for(const [x,h] of [[48,54],[100,72],[150,54],[200,54],[250,54]])pl.push(rect(x,111,3,h,'#afcbd8'),line(x,111,x,110+h,C.white));
  for(const [x,label] of [[52,'x'],[62,'-'],[150,'+']])pl.push(rect(x,42,9,9,'#d8e5eb'),...pill(x,42,9,9,true,label==='x'?'#ec877a':'#7acb68'),text(x+2,44,label,C.deep,{spacing:0}));
  await draw('Aqua playlist shade and popup menus',pl,{layers:[]});
  await call('studio_atlas',{});
  await call('studio_canvas',{surface:'skin',active:true,pressed:false});
}
export async function call(name, args = {}) {
  const response = await fetch(connectionUrl(), {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: requestId++, method: 'tools/call', params: { name, arguments: args } }),
  });
  const envelope = await response.json();
  const result = envelope.result;
  const message = result?.content?.filter(c => c.type === 'text').map(c => c.text).join('\n') || '';
  if (envelope.error || result?.isError) throw new Error(JSON.stringify(envelope.error || message));
  let data;
  try { data = JSON.parse(message); } catch { data = { text: message }; }
  await fs.mkdir(path.join(review, 'reports'), { recursive: true });
  await fs.writeFile(path.join(review, 'reports', `${name}.json`), JSON.stringify(data, null, 2));
  return data;
}

if (process.argv[2] === 'tool') {
  const value = process.argv[4] || '{}';
  const args = JSON.parse(value.startsWith('@') ? await fs.readFile(value.slice(1), 'utf8') : value);
  console.log(JSON.stringify(await call(process.argv[3], args)));
}
if (process.argv[2] === 'paint') await paint();
if (process.argv[2] === 'refine') await finishDetails();
if (process.argv[2] === 'extras') await paintExtras();
