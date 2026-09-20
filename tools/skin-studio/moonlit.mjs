import { pathToFileURL } from 'node:url';
import path from 'node:path';

export const ROOT = path.resolve(import.meta.dirname, '../..');
export async function call(name, args = {}) {
  const response = await fetch('http://127.0.0.1:18765/mcp', {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name, arguments: args } }),
  });
  const message = await response.json();
  if (!response.ok || message.error || message.result?.isError) throw new Error(JSON.stringify(message));
  const text = message.result.content.find(part => part.type === 'text')?.text;
  try { return JSON.parse(text); } catch { return text; }
}

export const C = {
  ink: '#171c30', deep: '#20283f', panel: '#29344a', edge: '#435269', steel: '#72838d',
  goldDark: '#795b50', gold: '#c09268', goldLight: '#ebc991', cream: '#fff0ca',
  furShade: '#8a919a', fur: '#c5c6bc', white: '#e9e4d0', pink: '#c88887',
  jade: '#86bca9', shadow: '#101626', patch: '#485568',
};
export class Pen {
  constructor() { this.ops = []; }
  rect(x,y,width,height,color) { this.ops.push({op:'rect',x,y,width,height,color,fill:true});return this; }
  line(x,y,x2,y2,color,brush_size=1) { this.ops.push({op:'line',x,y,x2,y2,color,brush_size});return this; }
  poly(points,color) { this.ops.push({op:'path',x:0,y:0,points,color,fill:true});return this; }
  oval(x,y,width,height,color) { this.ops.push({op:'ellipse',x,y,width,height,color,fill:true});return this; }
  curve(x,y,cx,cy,x2,y2,color,brush_size=1) { this.ops.push({op:'curve',x,y,x2,y2,control:[cx,cy],color,brush_size,clean_corners:false});return this; }
  stamp(x,y,rows,palette) { this.ops.push({op:'stamp',x,y,rows,palette});return this; }
  text(x,y,text,color,face='small',spacing=1) { this.ops.push({op:'text',x,y,text,color,face,spacing});return this; }
  star(x,y,color=C.goldLight) { return this.stamp(x-2,y-2,['  x  ','  x  ','xxxxx','  x  ','  x  '],{x:color}); }
  box(x,y,w,h) {
    this.poly([[x+3,y],[x+w-4,y],[x+w-1,y+3],[x+w-1,y+h-4],[x+w-4,y+h-1],[x+3,y+h-1],[x,y+h-4],[x,y+3]],C.goldDark);
    this.rect(x+2,y+2,w-4,h-4,C.deep).line(x+4,y+1,x+w-5,y+1,C.gold).line(x+1,y+4,x+1,y+h-5,C.gold);
    return this;
  }
}
export async function layer(name) { return call('studio_layers',{action:'add',name}); }
export async function draw(p, args={}) { return call('studio_draw',{label:'Moonlit atelier',operations:p.ops,...args}); }
export async function canvas(view={}) { return call('studio_canvas',{presentation:false,guides:false,grid:false,clip:null,alpha_lock:false,mask_colors:[],...view}); }
export async function sprite(id,p,states='all') { return draw(p,{origin:id,states}); }


export async function foundation() {
  await layer('01 · Midnight enamel & brass');
  const p=new Pen().rect(0,0,275,232,C.ink).rect(6,14,263,218,C.deep);
  p.rect(7,72,261,42,C.panel).line(9,73,265,73,C.edge).line(9,110,265,110,C.shadow);
  for(const [x,col] of [[0,C.shadow],[1,C.goldDark],[2,C.gold],[3,C.goldLight],[4,C.goldDark],[5,C.ink],[269,C.goldDark],[270,C.gold],[271,C.goldLight],[272,C.goldDark],[273,C.shadow],[274,C.shadow]]) p.rect(x,0,1,232,col);
  p.box(13,18,93,51).rect(18,22,83,42,C.ink);
  p.box(109,22,154,34).rect(112,26,148,9,C.ink).rect(112,39,88,12,C.ink);
  p.line(117,53,249,53,C.edge).star(104,64,C.gold).text(112,17,'NOCTURNE / STEREO',C.gold, 'small',0);
  p.rect(15,154,247,64,C.ink).line(17,153,259,153,C.edge).line(17,218,259,218,C.goldDark);
  p.text(21,222,'PRE',C.gold,'small',0);
  ['60','170','310','600','1K','3K','6K','12K','14K','16K'].forEach((s,i)=>p.text(79+i*18,222,s,C.steel,'small',0));
  p.line(10,115,264,115,C.deep).line(10,231,264,231,C.deep);
  await draw(p,{layers:['main.background','equalizer.background'],states:'all'});
  const title=new Pen().rect(0,0,275,14,C.deep).line(9,1,265,1,C.goldDark).line(12,2,262,2,C.gold).line(16,3,258,3,C.goldLight);
  for(const [x,col] of [[0,C.shadow],[1,C.goldDark],[2,C.gold],[3,C.goldLight],[4,C.goldDark],[5,C.ink],[269,C.goldDark],[270,C.gold],[271,C.goldLight],[272,C.goldDark],[273,C.shadow],[274,C.shadow]]) title.rect(x,4,1,10,col);
  title.text(80,7,'CATAMP / MOONLIT',C.goldLight,'small',1).star(71,9,C.jade);
  await sprite('main.title',title);
  const eqTitle=new Pen().rect(0,0,275,14,C.deep);
  for(const [x,col] of [[0,C.shadow],[1,C.goldDark],[2,C.gold],[3,C.goldLight],[4,C.goldDark],[5,C.ink],[269,C.goldDark],[270,C.gold],[271,C.goldLight],[272,C.goldDark],[273,C.shadow],[274,C.shadow]]) eqTitle.rect(x,0,1,14,col);
  eqTitle.text(16,6,'THE MIDNIGHT CLUB',C.gold,'small',0).star(222,8,C.goldLight);
  await sprite('equalizer.title',eqTitle);
  for(const [id,w] of [['playlist.top.left',25],['playlist.top.tile',25],['playlist.title',100],['playlist.top.right',25]]) {
    const q=new Pen().rect(0,0,w,20,C.deep).line(0,18,w-1,18,C.edge);
    if(id==='playlist.title') q.text(18,8,'AFTER HOURS',C.goldLight,'small',1).star(8,10,C.jade).star(89,10,C.jade);
    if(id==='playlist.top.left') for(const[x,col]of [[0,C.shadow],[1,C.goldDark],[2,C.gold],[3,C.goldLight],[4,C.goldDark],[5,C.ink]])q.rect(x,0,1,20,col);
    if(id==='playlist.top.right') for(const[x,col]of [[19,C.goldDark],[20,C.gold],[21,C.goldLight],[22,C.goldDark],[23,C.shadow],[24,C.shadow]])q.rect(x,0,1,20,col);
    await sprite(id,q);
  }
  for(const[id,w,right]of [['playlist.left.rail',12,false],['playlist.right.rail',20,true]]){
    const q=new Pen().rect(0,0,w,29,C.deep);
    const colors=right?[[14,C.goldDark],[15,C.gold],[16,C.goldLight],[17,C.goldDark],[18,C.shadow],[19,C.shadow]]:[[0,C.shadow],[1,C.goldDark],[2,C.gold],[3,C.goldLight],[4,C.goldDark],[5,C.ink]];
    for(const[x,col]of colors)q.rect(x,0,1,29,col);
    if(right) q.rect(3,0,9,29,C.ink).rect(3,0,1,29,C.edge).rect(11,0,1,29,C.edge);
    await sprite(id,q);
  }
  const f=new Pen().rect(0,339,275,38,C.deep).rect(6,342,263,29,C.panel);
  for(const[x,col]of [[0,C.shadow],[1,C.goldDark],[2,C.gold],[3,C.goldLight],[4,C.goldDark],[5,C.ink],[269,C.goldDark],[270,C.gold],[271,C.goldLight],[272,C.goldDark],[273,C.shadow],[274,C.shadow]])f.rect(x,339,1,35,col);
  f.line(6,372,268,372,C.goldDark).line(5,373,269,373,C.gold).line(5,374,269,374,C.goldLight).line(3,375,271,375,C.goldDark).line(1,376,273,376,C.shadow);
  f.box(128,346,97,26).rect(132,349,88,9,C.ink).rect(191,363,31,8,C.ink);
  for(const[x,w]of [[10,28],[39,28],[69,28],[99,36],[228,28]])f.box(x+2,348,w-4,15);
  f.text(19,353,'+',C.cream).text(48,353,'-',C.cream).text(76,353,'OK',C.cream,'small',0).text(106,353,'...',C.cream).line(239,352,247,352,C.cream).line(239,355,247,355,C.cream).line(239,358,247,358,C.cream);
  for(const [x,shape]of [[139,'|<'],[148,'>'],[157,'||'],[166,'#'],[175,'>|'],[185,'^']])f.text(x,365,shape,C.goldLight,'small',-1);
  f.line(261,367,266,362,C.gold).line(261,371,268,364,C.goldLight);
  await draw(f,{layers:['playlist.bottom.left','playlist.bottom.right'],states:'all'});
}


function placed(p,x,y) {
  for(const op of p.ops){op.x=(op.x||0)+x;op.y=(op.y||0)+y;if(op.x2!==undefined)op.x2+=x;if(op.y2!==undefined)op.y2+=y;if(op.control)op.control=[op.control[0]+x,op.control[1]+y];}
  return p;
}
export function sittingCat(x,y) {
  const p=new Pen();
  // Tail is behind the haunch; its silhouette is continuous, not a string of dots.
  p.curve(23,45,42,52,33,31,C.shadow,7).curve(23,44,40,49,33,31,C.furShade,5).curve(26,44,36,46,34,35,C.fur,3);
  p.line(31,42,36,42,C.patch,2).line(33,36,36,35,C.patch,2);
  p.poly([[13,20],[24,22],[26,31],[30,42],[27,49],[7,50],[2,46],[3,38],[8,31]],C.shadow);
  p.poly([[13,22],[23,24],[23,31],[28,42],[25,48],[7,48],[4,44],[6,36],[10,32]],C.furShade);
  p.poly([[13,23],[20,23],[22,30],[20,39],[22,48],[9,48],[7,44],[11,33]],C.fur);
  p.poly([[13,24],[20,24],[20,28],[23,30],[19,30],[20,33],[16,32],[16,36],[12,32],[10,30]],C.white);
  p.poly([[7,36],[12,34],[11,42],[8,46],[6,45],[6,41]],C.patch);
  p.line(13,35,12,48,C.white,3).line(20,34,21,48,C.white,3).line(15,38,15,48,C.furShade);
  p.rect(8,48,8,3,C.white).rect(19,48,8,3,C.white).line(9,50,14,50,C.furShade).line(20,50,25,50,C.furShade);
  p.rect(10,49,1,1,C.patch).rect(13,49,1,1,C.patch).rect(21,49,1,1,C.patch).rect(24,49,1,1,C.patch);
  p.poly([[4,0],[12,6],[21,6],[28,1],[27,13],[30,18],[25,23],[20,26],[13,26],[6,22],[2,18],[4,12]],C.shadow);
  p.poly([[5,2],[12,8],[21,8],[27,3],[25,14],[28,18],[23,22],[20,24],[13,24],[7,20],[4,17],[6,12]],C.fur);
  p.poly([[6,4],[11,9],[7,11]],C.pink).poly([[24,7],[25,5],[24,12],[21,11]],C.pink);
  p.poly([[6,12],[10,8],[14,9],[13,15],[10,18],[5,16]],C.patch);
  p.poly([[19,8],[23,9],[25,13],[23,17],[21,15],[20,12]],C.furShade);
  p.poly([[14,10],[18,10],[19,16],[22,20],[19,23],[14,23],[10,19],[14,17]],C.white);
  p.line(6,15,11,16,C.ink).line(20,16,25,14,C.ink).rect(8,16,3,1,C.jade).rect(21,16,3,1,C.jade).rect(10,15,1,2,C.ink).rect(21,15,1,2,C.ink);
  p.rect(16,19,3,1,C.pink).rect(17,20,1,1,C.goldDark).line(17,21,15,22,C.patch).line(17,21,19,22,C.patch);
  p.line(8,20,0,18,C.white).line(7,22,1,23,C.fur).line(25,20,33,18,C.white).line(25,22,32,23,C.fur);
  p.rect(14,9,2,1,C.cream).rect(6,12,1,2,C.white).line(9,7,11,8,C.cream).rect(23,8,1,2,C.cream);
  return placed(p,x,y);
}
export function sleepingCat(x,y) {
  const p=new Pen();
  p.curve(39,11,55,15,46,24,C.shadow,7).curve(39,10,53,15,46,23,C.furShade,5).curve(41,11,51,15,47,20,C.fur,2);
  p.oval(13,2,33,19,C.shadow).oval(14,3,30,16,C.furShade).oval(16,3,25,13,C.fur).oval(18,4,20,10,C.white);
  p.poly([[22,4],[26,3],[25,8],[22,12],[18,13],[21,8]],C.patch).poly([[32,4],[36,5],[34,10],[30,13],[27,13],[31,8]],C.furShade).poly([[40,8],[43,11],[40,16],[37,17],[39,12]],C.patch);
  p.poly([[1,0],[8,5],[12,4],[18,0],[17,10],[20,14],[15,19],[6,19],[0,14],[2,8]],C.shadow);
  p.poly([[2,2],[8,7],[12,6],[17,2],[15,10],[18,14],[14,17],[6,17],[2,13],[4,8]],C.fur);
  p.poly([[3,4],[7,8],[4,9]],C.pink).poly([[14,7],[16,4],[14,10]],C.pink).poly([[6,7],[10,8],[9,12],[6,13],[3,11]],C.patch);
  p.poly([[10,9],[12,9],[13,12],[16,14],[13,17],[9,17],[7,14]],C.white);
  p.line(4,12,7,13,C.ink).line(12,13,15,12,C.ink).rect(9,14,2,1,C.pink).rect(10,15,1,1,C.patch);
  p.line(3,15,0,14,C.white).line(15,15,20,14,C.white);
  p.oval(7,17,18,4,C.shadow).rect(8,17,16,2,C.white).line(10,19,22,19,C.furShade).rect(12,18,1,1,C.furShade).rect(16,18,1,1,C.furShade);
  p.line(25,3,30,3,C.cream).rect(19,5,2,1,C.cream);
  return placed(p,x,y);
}
export async function illustrations() {
  await layer('02 · Luna, Miso & the crescent');
  const p=new Pen().oval(18,22,28,38,C.goldDark).oval(19,22,26,36,C.gold).oval(20,22,23,34,C.goldLight).oval(26,19,24,33,C.ink);
  p.line(22,33,20,41,C.cream).rect(22,47,1,3,C.cream).star(35,23,C.cream).star(102,18,C.gold).star(102,62,C.jade).rect(47,21,1,1,C.steel).rect(63,65,1,1,C.gold).rect(72,20,1,1,C.gold);
  p.text(36,63,'GOODNIGHT',C.gold,'small',0);
  await draw(p,{layers:['main.background']});
  await draw(sittingCat(40,165),{layers:['equalizer.background']});
  await draw(sleepingCat(147,112),{layers:['main.background','equalizer.background','equalizer.title'],states:'all'});
  const s=new Pen().star(118,122,C.goldLight).rect(127,117,1,1,C.steel).rect(216,124,1,1,C.jade).text(212,148,'Z Z',C.gold,'small',0);
  await draw(s,{layers:['equalizer.background','equalizer.title'],states:'all'});
}


function key(w,h,held=false,bg=C.deep) {
  const p=new Pen().rect(0,0,w,h,bg).box(1,1,w-2,h-2);
  p.rect(3,3,w-6,h-6,held?C.goldDark:C.panel).line(4,2,w-5,2,held?C.goldDark:C.goldLight);
  if(!held)p.line(4,h-4,w-5,h-4,C.ink);
  return p;
}
export async function controls() {
  await layer('03 · Keys, paw faders & switch states');
  for(const held of [false,true]) {
    await canvas({pressed:held});
    for(const [name,w,h]of [['previous',23,18],['play',23,18],['pause',23,18],['stop',23,18],['next',22,18],['eject',22,16]]){
      const p=key(w,h,held,C.panel),dy=held?1:0,ink=held?C.cream:C.goldLight;
      if(name==='play')p.poly([[9,5+dy],[9,12+dy],[15,9+dy]],ink);
      if(name==='previous')p.rect(6,5+dy,2,8,ink).poly([[15,5+dy],[15,12+dy],[9,9+dy]],ink);
      if(name==='next')p.rect(15,5+dy,2,8,ink).poly([[7,5+dy],[7,12+dy],[13,9+dy]],ink);
      if(name==='pause')p.rect(8,5+dy,2,8,ink).rect(13,5+dy,2,8,ink);
      if(name==='stop')p.rect(8,6+dy,7,7,ink);
      if(name==='eject')p.poly([[7,8+dy],[11,4+dy],[15,8+dy]],ink).rect(7,10+dy,9,2,ink);
      await sprite('main.'+name,p,'current');
    }
    for(const[id,w,h,word]of [['equalizer.presets',44,12,'PRESETS'],['main.position.thumb',29,10,''],['main.volume.thumb',14,11,''],['main.balance.thumb',14,11,''],['playlist.scroll.thumb',8,18,'']]){
      const p=key(w,h,held,id.startsWith('main.position')?C.panel:C.deep);
      if(word)p.text(5,4+(held?1:0),word,C.cream,'small',0);
      else if(w===29)p.stamp(10,3,[' x x x ',' xxxxx ','  xxx  '],{x:C.cream});
      else if(w===8)p.line(3,6,3,11,C.goldLight).line(5,6,5,11,C.gold);
      else p.stamp(4,3,[' x x ','xxxxx',' xxx ','  x  '],{x:C.cream});
      await sprite(id,p,'current');
    }
    const paw=new Pen().rect(0,0,11,11,C.ink).stamp(0,0,['   ggggg   ','  glllllg  ',' glppppplg ','glppwpwpplg','glpwwwwwplg','glppwwwpplg',' glppwpp lg',' glppppplg ','  glllllg  ','   ggggg   ','           '],{g:C.goldDark,l:held?C.gold:C.goldLight,p:held?C.goldDark:C.deep,w:C.cream});
    await sprite('equalizer.band0.thumb',paw,'current');
    for(const[id,kind]of [['main.options','paw'],['main.minimize','min'],['main.shade','up'],['main.close','x'],['equalizer.close','x']]){
      const q=new Pen().rect(0,0,9,9,C.deep),col=held?C.cream:C.gold;
      if(kind==='min')q.line(2,5,6,5,col);
      if(kind==='up')q.line(2,5,4,3,col).line(4,3,6,5,col);
      if(kind==='x')q.line(3,3,5,5,col).line(3,5,5,3,col);
      if(kind==='paw')q.stamp(1,2,[' x x x ',' xxxxx ','  xxx  '],{x:col});
      await sprite(id,q,'current');
    }
    for(const active of [false,true]){
      await canvas({active,pressed:held});
      for(const[id,w,word]of [['main.shuffle',47,'SHUF'],['main.repeat',28,'REP'],['main.eq.toggle',23,'EQ'],['main.playlist.toggle',23,'PL'],['equalizer.on',26,'ON'],['equalizer.auto',32,'AUTO']]){
        const h=id==='main.shuffle'||id==='main.repeat'?15:12,p=key(w,h,held,id.startsWith('main.')&&h===15?C.panel:C.deep);
        p.text(7,h===15?5:4,word,active?C.cream:C.steel,'small',0).rect(3,h===15?7:5,2,2,active?C.jade:C.goldDark);
        await sprite(id,p,'current');
      }
      for(const[id,w,word]of [['main.mono',27,'MONO'],['main.stereo',29,'STEREO']]){
        const p=new Pen().rect(0,0,w,12,C.deep).text(0,5,word,active?C.jade:C.steel,'small',0);
        await sprite(id,p,'current');
      }
    }
  }
  await canvas({active:true,pressed:false});
  await sprite('main.position.track',new Pen().rect(0,0,248,10,C.panel).rect(2,3,244,4,C.ink).line(4,3,243,3,C.goldDark).line(4,7,243,7,C.edge));
  await sprite('main.volume.track',new Pen().rect(0,0,68,13,C.deep).rect(2,4,64,5,C.ink).line(3,9,65,9,C.edge).rect(3,5,[1,61],3,C.jade));
  await sprite('main.balance.track',new Pen().rect(0,0,38,13,C.deep).rect(2,4,34,5,C.ink).line(3,9,35,9,C.edge).rect(18,5,2,3,C.gold));
  await sprite('equalizer.band0.track',new Pen().rect(0,0,14,63,C.ink).rect(4,1,6,61,C.deep).line(5,2,5,60,C.edge).line(8,2,8,60,C.shadow).rect(6,[3,56],2,4,C.jade));
  await sprite('equalizer.graph',new Pen().rect(0,0,113,19,C.ink).line(0,0,112,0,C.goldDark).line(0,18,112,18,C.edge).line(1,9,111,9,C.panel));
  await sprite('equalizer.preamp.line',new Pen().rect(0,0,113,1,C.jade));
}


export async function typeAndPalette() {
  await layer('04 · Warm phosphor & night playlist');
  for(const [sheet,w,h]of [['text.bmp',155,18],['numbers.bmp',99,13]]) {
    await call('studio_atlas',{sheet});
    const clip=await call('studio_cluster',{rect:[0,0,w,h]});
    const palette=Object.fromEntries(Object.entries(clip.palette).map(([key,color])=>[key,color.toLowerCase()==='#254c67'?C.cream:C.ink]));
    await draw(new Pen().stamp(0,0,clip.rows,palette));
  }
  await canvas({active:true,pressed:false});
  await call('studio_targets',{auto:true});
  await draw(new Pen().rect(72,29,2,2,C.goldLight).rect(72,35,2,2,C.goldLight),{layers:['main.background']});
  for(let playback=0;playback<3;playback++){
    await canvas({playback});
    const p=new Pen().rect(0,0,9,9,C.ink);
    if(playback===0)p.rect(3,3,4,4,C.jade);
    if(playback===1)p.poly([[2,2],[2,7],[7,4]],C.jade);
    if(playback===2)p.rect(2,2,2,6,C.goldLight).rect(5,2,2,6,C.goldLight);
    await sprite('main.status',p,'current');
  }
  await call('studio_options',{
    playlist_colors:{Normal:C.fur,Current:C.cream,NormalBG:C.ink,SelectedBG:C.edge,MbFG:C.cream,MbBG:C.deep},
    visualizer_colors:[C.ink,C.panel,C.goldLight,C.goldLight,C.gold,C.gold,C.goldDark,C.jade,C.jade,C.jade,C.steel,C.steel,C.edge,C.edge,C.panel,C.panel,C.deep,C.deep,C.cream,C.jade,C.goldLight,C.gold,C.goldDark,C.panel],
  });
  await call('studio_cursors',{action:'draw',overwrite:true,style:'chisel'});
  await canvas({active:true,pressed:false,playback:0,position:10,volume:20,balance:14,scroll:5,eq:[14,23,18,13,9,6,9,13,17,21,24],zoom:2});
  await call('studio_targets',{auto:true});
}

export async function polish() {
  await canvas({active:true,pressed:false});
  await layer('05 · Engraving & finishing light');
  const f=new Pen().rect(138,364,59,8,C.deep);
  f.rect(140,365,1,5,C.goldLight).poly([[146,365],[146,369],[142,367]],C.goldLight);
  f.poly([[149,365],[149,369],[154,367]],C.goldLight);
  f.rect(158,365,1,5,C.goldLight).rect(161,365,1,5,C.goldLight);
  f.rect(167,365,4,5,C.goldLight);
  f.poly([[176,365],[176,369],[180,367]],C.goldLight).rect(182,365,1,5,C.goldLight);
  f.poly([[187,367],[190,364],[193,367]],C.goldLight).line(187,370,193,370,C.goldLight);
  await draw(f,{layers:['playlist.bottom.left','playlist.bottom.right']});
  const p=new Pen().line(16,111,123,111,C.edge).line(16,112,49,112,C.goldDark).rect(16,111,4,1,C.gold).line(228,111,258,111,C.edge).rect(255,111,4,1,C.gold);
  p.line(10,23,10,62,C.edge).rect(10,24,1,4,C.goldDark).line(265,24,265,47,C.edge);
  p.rect(209,126,1,1,C.gold).rect(228,122,1,1,C.jade);
  await draw(p,{layers:['main.background','equalizer.background']});
}

export async function finish() {
  const target=path.join(ROOT,'assets/skins/Catamp Moonlit');
  console.log(await call('studio_validate'));
  const exported=await call('studio_export',{path:target+'.wsz'});
  if(exported.hard_to_read?.length) throw new Error(JSON.stringify(exported.hard_to_read));
  console.log(exported);
  console.log(await call('studio_project',{action:'save',path:target+'.cstudio'}));
  console.log(await call('studio_screenshot',{presentation:true,panel:'all',path:target+'.png'}));
}

export async function build() {
  const before=await call('studio_status');
  if(before.dirty) throw new Error('Save or export the current project before replaying Moonlit.');
  await call('studio_open',{path:path.join(ROOT,'assets/skins/Catamp Silverplay.wsz')});
  await canvas({zoom:2});
  await foundation();
  await illustrations();
  await controls();
  await typeAndPalette();
  await polish();
  await finish();
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await build();
