import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { ROOT, Pen, call, canvas, sprite } from './moonlit.mjs';

export const P = {
  paper:'#f5e6cb', ink:'#493448', soft:'#b4a4a1', blush:'#f0b6b0',
  rose:'#e58b95', roseShade:'#ba667b', roseLight:'#ffc0b7',
  orange:'#eaa365', orangeShade:'#bb6c50', gold:'#ffd18a',
  mint:'#b9d3bc', mintShade:'#71988e', mintLight:'#d6e4c7',
  violet:'#82758e', pale:'#d7c3cb', cream:'#fff5df',
};
const STATIC=['main.background','main.title','equalizer.background','equalizer.title',
  'playlist.top.left','playlist.top.tile','playlist.title','playlist.top.right',
  'playlist.left.rail','playlist.right.rail','playlist.bottom.left','playlist.bottom.right'];
const UPPER=STATIC.slice(0,4);
export const paint=(p,args={})=>call('studio_draw',{label:'Catnip takeover',operations:p.ops,states:'all',...args});
const plane=name=>call('studio_layers',{action:'add',name});
function ellipse(p,x,y,w,h,fill,shade=fill) {
  p.oval(x,y,w,h,P.ink).oval(x+2,y+2,w-4,h-4,shade).oval(x+3,y+2,w-7,h-7,fill);
  return p;
}
function star(p,x,y,color=P.roseShade) {p.star(x,y,color);return p;}
function paw(p,x,y,fill=P.roseLight) {
  p.oval(x,y+2,15,9,P.ink).oval(x+1,y+2,13,7,fill);
  p.line(x+5,y+3,x+5,y+6,P.roseShade).line(x+9,y+3,x+9,y+6,P.roseShade);
  return p;
}
function fish(p,x,y,fill=P.orange,icon='') {
  p.poly([[x,y+1],[x+6,y+5],[x,y+10]],P.ink);
  p.poly([[x+1,y+3],[x+5,y+5],[x+1,y+8]],fill);
  ellipse(p,x+4,y,17,12,fill);
  p.rect(x+17,y+4,1,2,P.ink).line(x+9,y+1,x+12,y-1,P.ink);
  if(icon==='play')p.poly([[x+9,y+4],[x+9,y+8],[x+13,y+6]],P.ink);
  if(icon==='pause')p.rect(x+9,y+4,1,5,P.ink).rect(x+12,y+4,1,5,P.ink);
  if(icon==='stop')p.rect(x+9,y+4,4,4,P.ink);
  if(icon==='previous')p.line(x+8,y+4,x+8,y+8,P.ink).poly([[x+13,y+4],[x+13,y+8],[x+9,y+6]],P.ink);
  if(icon==='next')p.line(x+13,y+4,x+13,y+8,P.ink).poly([[x+8,y+4],[x+8,y+8],[x+12,y+6]],P.ink);
  if(icon==='eject')p.poly([[x+8,y+6],[x+11,y+3],[x+14,y+6]],P.ink).line(x+8,y+8,x+14,y+8,P.ink);
  return p;
}
export async function ground() {
  await plane('01 · One sheet of catnip paper');
  await paint(new Pen().rect(0,0,275,377,P.paper),{layers:STATIC});
  await call('studio_options',{
    playlist_colors:{Normal:P.ink,Current:P.ink,NormalBG:P.paper,SelectedBG:P.mint,MbFG:P.ink,MbBG:P.paper},
    visualizer_colors:[P.mint,P.mint,P.ink,P.ink,P.roseShade,P.roseShade,P.rose,P.rose,P.orangeShade,P.orangeShade,P.orange,P.orange,P.orange,P.gold,P.gold,P.gold,P.mintShade,P.mintShade,P.ink,P.roseShade,P.rose,P.orangeShade,P.gold,P.ink]
  });
  const p=new Pen();
  p.text(120,5,'CATNIP FM',P.ink,'small',1).text(119,17,'ZERO THOUGHTS. ALL VIBES.',P.roseShade,'small',0);
  p.line(121,12,164,12,P.rose,1).rect(204,5,3,3,P.orange).rect(210,8,2,2,P.mintShade);
  for(const [x,y,c]of [[7,75,P.rose],[112,54,P.orangeShade],[266,28,P.mintShade],[10,218,P.mintShade],[264,225,P.roseShade]])star(p,x,y,c);
  await paint(p,{layers:STATIC});
}
export async function clockCat() {
  await plane('02 · The cat is late again');
  const p=new Pen();
  p.poly([[8,15],[26,21],[78,17],[106,7],[104,40],[109,50],[102,64],[88,70],[24,70],[9,59],[5,43]],P.ink);
  p.poly([[11,19],[27,25],[79,21],[102,12],[100,41],[105,50],[99,60],[86,67],[25,66],[13,57],[9,43]],P.mintShade);
  p.poly([[12,20],[29,27],[80,23],[100,15],[98,44],[102,51],[95,60],[81,65],[27,63],[14,56],[11,43]],P.mint);
  p.poly([[14,24],[25,29],[14,36]],P.rose).poly([[86,24],[98,18],[96,33]],P.rose);
  p.poly([[44,22],[53,23],[51,30],[47,34]],P.mintShade).poly([[67,21],[76,22],[72,29],[67,30]],P.mintShade);
  // The digits are the right eye. The spectrum is the mouth.
  p.curve(16,35,25,25,35,35,P.ink,2).line(16,36,18,32,P.ink);
  p.rect(18,40,11,3,P.blush).line(11,43,2,39,P.ink).line(10,48,1,48,P.ink);
  p.line(101,47,113,42,P.ink).line(101,51,113,54,P.ink);
  p.poly([[47,58],[60,58],[60,65],[56,68],[51,68],[47,65]],P.ink);
  p.rect(49,59,9,6,P.rose).line(54,61,54,65,P.roseShade);
  p.text(16,7,'LATE AGAIN',P.ink,'small',0);
  await paint(p,{layers:UPPER});
}
export async function noodleCat() {
  await plane('03 · Noodle cat spills across the join');
  const p=new Pen();
  // One silhouette crosses y=116. The title strip receives the same native marks.
  p.curve(32,99,-12,89,7,70,P.ink,9).curve(32,99,-9,89,7,70,P.rose,5);
  p.poly([[8,92],[17,85],[40,83],[183,83],[202,89],[230,91],[247,104],[250,119],[240,130],[214,132],[181,128],[49,128],[21,122],[10,113]],P.ink);
  p.poly([[11,93],[20,88],[44,86],[182,86],[204,93],[229,94],[244,105],[247,118],[238,127],[215,129],[181,125],[50,125],[24,119],[13,111]],P.roseShade);
  p.poly([[12,93],[22,88],[45,87],[183,87],[204,95],[230,96],[241,106],[243,116],[234,121],[212,124],[179,122],[52,122],[26,116],[14,109]],P.rose);
  p.line(27,88,93,88,P.roseLight).line(33,119,144,119,P.roseLight);
  // Floppy head, one evil eye, one delighted eye, tongue out.
  p.poly([[206,89],[202,68],[223,82],[244,82],[266,69],[264,94],[270,106],[266,123],[254,133],[223,134],[208,122],[201,108]],P.ink);
  p.poly([[209,89],[206,73],[224,86],[245,86],[263,74],[261,96],[267,107],[263,122],[252,130],[224,131],[211,120],[205,108]],P.roseShade);
  p.poly([[210,89],[209,78],[224,89],[245,88],[260,79],[257,98],[264,108],[259,120],[250,127],[226,128],[214,118],[209,107]],P.rose);
  p.poly([[212,82],[222,89],[213,93]],P.roseLight).poly([[250,89],[258,83],[256,95]],P.roseLight);
  p.poly([[227,87],[232,88],[230,96],[227,98]],P.roseShade).poly([[239,87],[245,87],[241,95],[237,97]],P.roseShade);
  p.line(214,105,229,108,P.ink,2).oval(238,101,16,13,P.cream).oval(243,104,5,7,P.ink);
  p.rect(216,113,9,2,P.blush).rect(251,116,7,2,P.blush);
  p.poly([[230,115],[237,115],[234,119]],P.ink);
  p.curve(222,121,233,128,243,122,P.ink,2);
  p.poly([[232,124],[239,124],[239,133],[235,137],[231,133]],P.ink);
  p.rect(233,125,4,8,P.roseLight).line(235,127,235,132,P.roseShade);
  p.line(211,113,197,108,P.ink).line(211,118,197,120,P.ink);
  p.line(258,113,273,108,P.ink).line(259,120,273,125,P.ink);
  // Paws grab the imaginary seam; no horizontal edge belongs to a panel.
  paw(p,39,118);paw(p,145,119);
  p.line(60,111,64,118,P.roseShade,2).line(78,111,81,118,P.roseShade,2);
  p.text(88,113,'I ATE THE UI',P.ink,'small',0);
  await paint(p,{layers:UPPER});
}
export async function playlistCat() {
  await plane('04 · The playlist has been claimed');
  const p=new Pen();
  // Long tail reaches up into the EQ; head crosses the EQ/playlist join.
  p.curve(113,228,42,247,48,183,P.ink,12).curve(111,228,46,243,49,183,P.violet,8);
  p.curve(49,184,57,163,68,176,P.ink,12).curve(49,184,57,167,68,176,P.violet,8);
  p.line(45,198,53,199,P.ink,2).line(51,218,60,216,P.ink,2).line(77,230,79,224,P.ink,2);
  p.poly([[100,228],[98,213],[115,222],[151,221],[172,210],[170,230],[180,239],[172,248],[107,249],[95,241]],P.ink);
  p.poly([[103,229],[102,218],[116,226],[152,225],[168,215],[166,231],[176,239],[170,245],[109,246],[99,240]],P.violet);
  p.poly([[116,226],[129,225],[139,231],[151,225],[164,228],[169,239],[159,246],[116,246],[105,239]],P.pale);
  p.poly([[104,221],[113,227],[105,231]],P.rose).poly([[157,226],[166,218],[164,230]],P.rose);
  p.line(111,233,122,236,P.ink,2).line(149,236,160,232,P.ink,2);
  p.poly([[132,236],[141,236],[137,240]],P.roseShade);
  p.line(137,240,133,243,P.ink).line(137,240,142,243,P.ink);
  p.line(110,239,94,235,P.ink).line(109,243,92,245,P.ink);
  p.line(163,239,181,233,P.ink).line(164,243,183,246,P.ink);
  p.text(7,238,'MINE.',P.ink,'small',1).text(205,222,'MINE TOO.',P.ink,'small',0);
  await paint(p,{layers:[...UPPER,'playlist.top.left','playlist.title','playlist.top.right']});
  // Footer is a sleeping orange cat. Text and glyphs sit in its coat.
  const f=new Pen();
  f.curve(229,364,277,382,268,347,P.ink,12).curve(229,364,272,378,266,347,P.orange,8);
  f.oval(59,345,171,36,P.ink).oval(61,347,167,32,P.orangeShade).oval(63,347,157,26,P.orange);
  f.poly([[11,349],[9,337],[28,345],[47,345],[64,335],[63,355],[69,366],[59,376],[19,376],[6,365]],P.ink);
  f.poly([[14,350],[13,342],[29,349],[48,349],[60,340],[59,356],[65,366],[57,373],[20,373],[10,364]],P.gold);
  f.poly([[15,346],[24,351],[17,355]],P.rose).poly([[50,351],[58,345],[56,356]],P.rose);
  f.line(16,362,27,365,P.ink,2).line(43,365,54,361,P.ink,2);
  f.poly([[32,365],[40,365],[36,369]],P.ink);
  f.line(12,368,1,365,P.ink).line(59,369,70,365,P.ink);
  f.poly([[88,347],[97,347],[90,356],[82,361],[83,354]],P.orangeShade);
  f.poly([[109,348],[118,348],[111,358],[103,361]],P.orangeShade);
  f.poly([[216,350],[223,355],[216,362],[208,364],[210,358]],P.orangeShade);
  f.text(83,361,'Z Z Z',P.ink,'small',0);
  f.line(246,354,259,351,P.ink).line(249,360,263,357,P.ink);
  await paint(f,{layers:['playlist.bottom.left','playlist.bottom.right']});
}
export async function toys() {
  await plane('05 · Fish, yarn, and absolutely no bezels');
  for(const held of [false,true]) {
    await canvas({active:true,pressed:held});
    for(const [name,w,h]of [['previous',23,18],['play',23,18],['pause',23,18],['stop',23,18],['next',22,18],['eject',22,16]]) {
      const p=new Pen().rect(0,0,w,h,P.rose);
      fish(p,1,3+(held?1:0),held?P.gold:P.cream,name);
      await sprite('main.'+name,p,'current');
    }
    let p=new Pen().rect(0,0,29,10,P.paper);
    // A tiny mouse runs along the yarn seek line.
    p.curve(3,6,0,0,8,2,P.roseShade).oval(7,3,17,6,P.ink).oval(8,3,14,4,held?P.rose:P.pale).oval(14,0,6,6,P.ink).oval(15,1,4,3,P.rose).rect(22,4,2,1,P.ink);
    await sprite('main.position.thumb',p,'current');
    for(const id of ['main.volume.thumb','main.balance.thumb']) {
      p=new Pen().rect(0,0,14,11,P.paper);
      ellipse(p,2,0,10,10,held?P.rose:P.mint);
      p.curve(4,2,12,3,4,7,P.ink).line(3,5,10,9,P.ink);
      await sprite(id,p,'current');
    }
    p=new Pen().rect(0,0,11,11,P.paper);
    p.poly([[1,1],[5,3],[9,1],[10,4],[9,8],[6,10],[2,9],[0,6]],P.ink);
    p.poly([[2,3],[5,5],[8,3],[9,5],[8,8],[5,9],[2,8],[1,6]],held?P.rose:P.orange);
    p.rect(7,5,1,1,P.ink).line(3,5,3,8,P.orangeShade);
    await sprite('equalizer.band0.thumb',p,'current');
    p=new Pen().rect(0,0,8,18,P.paper);
    p.line(4,0,4,17,P.soft).oval(1,5,6,9,P.ink).oval(2,6,4,7,held?P.rose:P.orange).rect(4,7,1,1,P.ink);
    await sprite('playlist.scroll.thumb',p,'current');
    for(const [id,kind]of [['main.options','paw'],['main.minimize','min'],['main.shade','up'],['main.close','x'],['equalizer.close','x']]) {
      const q=new Pen().rect(0,0,9,9,id==='equalizer.close'?P.paper:P.paper),ink=held?P.roseShade:P.ink;
      if(kind==='min')q.line(2,5,6,5,ink);
      if(kind==='up')q.line(2,5,4,3,ink).line(4,3,6,5,ink);
      if(kind==='x')q.line(2,2,6,6,ink).line(2,6,6,2,ink);
      if(kind==='paw')q.stamp(1,2,[' x x x ',' xxxxx ','  xxx  '],{x:ink});
      await sprite(id,q,'current');
    }
    p=new Pen().rect(0,0,44,12,P.paper).text(6,4+(held?1:0),'FEED ME',held?P.roseShade:P.ink,'small',0);
    await sprite('equalizer.presets',p,'current');
    for(const active of [false,true]) {
      await canvas({active,pressed:held});
      for(const[id,w,h,word,bg]of [['main.shuffle',47,15,'SHUF',P.rose],['main.repeat',28,15,'REP',P.rose],['main.eq.toggle',23,12,'EQ',P.paper],['main.playlist.toggle',23,12,'PL',P.paper],['equalizer.on',26,12,'ON',P.paper],['equalizer.auto',32,12,'AUTO',P.paper]]) {
        const q=new Pen().rect(0,0,w,h,bg).text(5,h===15?5:4,word,active?P.ink:P.roseShade,'small',0);
        if(active)q.line(4,h-2,w-6,h-3,held?P.ink:P.roseShade);
        await sprite(id,q,'current');
      }
      for(const[id,w,word]of [['main.mono',27,'MONO'],['main.stereo',29,'STEREO']])await sprite(id,new Pen().rect(0,0,w,12,P.paper).text(0,5,word,active?P.ink:P.soft,'small',0),'current');
    }
  }
  await canvas({active:true,pressed:false});
  await sprite('main.position.track',new Pen().rect(0,0,248,10,P.paper).curve(1,4,93,8,244,4,P.roseShade).curve(119,6,122,-2,133,4,P.rose));
  await sprite('main.volume.track',new Pen().rect(0,0,68,13,P.paper).curve(2,6,31,11,65,6,P.soft).rect(3,6,[1,61],1,P.mintShade));
  await sprite('main.balance.track',new Pen().rect(0,0,38,13,P.paper).line(2,6,35,6,P.soft).rect([3,32],5,2,3,P.roseShade));
  await sprite('equalizer.band0.track',new Pen().rect(0,0,14,63,P.paper).curve(7,2,2,31,7,61,P.soft).rect(7,[3,56],1,3,P.roseShade));
  await sprite('equalizer.graph',new Pen().rect(0,0,113,19,P.paper));
  await sprite('equalizer.preamp.line',new Pen().rect(0,0,113,1,P.mintShade));
  // Footer actions are hand-lettered into the cat, with simple transport marks.
  const f=new Pen().text(18,349,'+',P.ink).text(47,349,'-',P.ink).text(77,352,'OK',P.ink,'small',0).text(107,352,'...',P.ink,'small',0);
  f.oval(127,345,99,17,P.paper).rect(132,347,89,12,P.paper).oval(189,361,36,12,P.paper);
  f.line(236,350,249,350,P.ink).line(238,353,251,353,P.ink).line(236,356,249,356,P.ink);
  f.rect(139,364,49,8,P.orange);
  f.line(140,365,140,369,P.ink).poly([[146,365],[142,367],[146,369]],P.ink);
  f.poly([[149,365],[154,367],[149,369]],P.ink).line(158,365,158,369,P.ink).line(161,365,161,369,P.ink);
  f.rect(167,365,4,5,P.ink).poly([[176,365],[180,367],[176,369]],P.ink).line(182,365,182,369,P.ink);
  f.poly([[187,367],[190,364],[193,367]],P.ink).line(187,370,193,370,P.ink);
  await paint(f,{layers:['playlist.bottom.left','playlist.bottom.right']});
}
export async function lettering() {
  await plane('06 · Ink and small mischief');
  for(const[sheet,w,h,bg]of [['text.bmp',155,18,P.paper],['numbers.bmp',99,13,P.mint]]) {
    await call('studio_atlas',{sheet});
    const clip=await call('studio_cluster',{rect:[0,0,w,h]});
    const palette=Object.fromEntries(Object.entries(clip.palette).map(([key,color])=>[key,color.toLowerCase()==='#254c67'?P.ink:bg]));
    await paint(new Pen().stamp(0,0,clip.rows,palette));
  }
  await canvas({active:true,pressed:false});
  await call('studio_targets',{auto:true});
  await paint(new Pen().rect(72,29,2,2,P.ink).rect(72,35,2,2,P.ink),{layers:['main.background']});
  for(let playback=0;playback<3;playback++) {
    await canvas({playback});
    const p=new Pen().rect(0,0,9,9,P.mint);
    if(playback===0)p.rect(3,3,4,4,P.ink);
    if(playback===1)p.poly([[2,2],[2,7],[7,4]],P.ink);
    if(playback===2)p.rect(2,2,2,6,P.ink).rect(5,2,2,6,P.ink);
    await sprite('main.status',p,'current');
  }
  await call('studio_cursors',{action:'draw',overwrite:true,style:'chisel'});
}
export async function joins() {
  await canvas({active:true,pressed:false});
  await plane('07 · Ears through the yarn, and finishing whiskers');
  // The stationary seek track shares the paper with the cat's ears.
  const p=new Pen();
  p.poly([[206,89],[202,68],[223,82],[244,82],[266,69],[264,94],[270,106],[266,123],[254,133],[223,134],[208,122],[201,108]],P.ink);
  p.poly([[209,89],[206,73],[224,86],[245,86],[263,74],[261,96],[267,107],[263,122],[252,130],[224,131],[211,120],[205,108]],P.roseShade);
  p.poly([[210,89],[209,78],[224,89],[245,88],[260,79],[257,98],[264,108],[259,120],[250,127],[226,128],[214,118],[209,107]],P.rose);
  await paint(p,{layers:['main.position.track']});
  const q=new Pen().text(13,218,'PURR',P.mintShade,'small',0);
  q.line(3,153,8,147,P.roseShade).line(3,160,8,160,P.roseShade);
  q.line(263,173,268,168,P.roseShade).line(266,178,271,178,P.roseShade);
  await paint(q,{layers:UPPER});
}
export async function finish() {
  await canvas({active:true,pressed:false,playback:0,position:10,volume:20,balance:14,scroll:5,eq:[14,23,18,13,9,6,9,13,17,21,24],preview_playlist_height:145,zoom:2,presentation:true});
  await call('studio_targets',{auto:true});
  const target=path.join(ROOT,'assets/skins/Catamp Catnip');
  const validation=await call('studio_validate');
  if(validation.divergences?.length)throw new Error(JSON.stringify(validation));
  console.log(validation);
  const exported=await call('studio_export',{path:target+'.wsz'});
  if(exported.hard_to_read?.length)throw new Error(JSON.stringify(exported.hard_to_read));
  console.log(exported);
  console.log(await call('studio_project',{action:'save',path:target+'.cstudio'}));
  console.log(await call('studio_screenshot',{presentation:true,panel:'all',path:target+'.png'}));
}
export async function build() {
  const before=await call('studio_status');
  if(before.dirty)throw new Error('Save the current project before replaying Catnip.');
  await call('studio_open',{path:path.join(ROOT,'assets/skins/Catamp Silverplay.wsz')});
  await canvas({zoom:2,preview_playlist_height:145});
  await ground();await clockCat();await noodleCat();await playlistCat();await toys();await lettering();await joins();await finish();
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)await build();
