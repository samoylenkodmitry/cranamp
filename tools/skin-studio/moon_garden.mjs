import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { ROOT, Pen, call, canvas, sprite } from './moonlit.mjs';

// Redrawn with native Studio geometry from the imagegen Moon Garden reference.
// No pixels are imported from the concept image.
export const C={
  night:'#151d31', blue:'#1f2b46', water:'#293e5b', reflection:'#465779',
  shadow:'#101729', stoneDeep:'#2c3655', stone:'#465173', stoneLight:'#64708d',
  vine:'#395963', leaf:'#587770', leafLight:'#819782', teal:'#9db4a0',
  furDeep:'#7d809f', furShade:'#adb0c9', fur:'#dcd8d6', ivory:'#f8e8c7', white:'#fff1d0',
  goldDark:'#a8815c', gold:'#d2ac72', lamp:'#f4cf8d', pink:'#ba999d', purple:'#8186b1',
};
const STATIC=['main.background','main.title','equalizer.background','equalizer.title','playlist.top.left','playlist.top.tile','playlist.title','playlist.top.right','playlist.left.rail','playlist.right.rail','playlist.bottom.left','playlist.bottom.right'];
const UPPER=STATIC.slice(0,4);
const CAT_TARGETS=[...UPPER,'main.previous','main.play','main.pause','main.stop','main.next','main.eject','equalizer.on','equalizer.auto','equalizer.graph'];
const paint=(p,args={})=>call('studio_draw',{label:'Moon Garden native atelier',operations:p.ops,states:'all',...args});
const plane=name=>call('studio_layers',{action:'add',name});
function leaf(p,x,y,w=8,h=10,side=1,color=C.leaf) {
  const sx=n=>x+n*side;
  p.poly([[sx(0),y],[sx(w*.45),y+1],[sx(w*.7),y-1],[sx(w),y+h*.3],[sx(w*.75),y+h*.38],[sx(w*.9),y+h*.63],[sx(w*.58),y+h*.65],[sx(w*.36),y+h],[sx(w*.16),y+h*.7],[sx(-1),y+h*.5],[sx(1),y+h*.3]],C.vine);
  p.poly([[sx(1),y+1],[sx(w*.5),y+2],[sx(w*.69),y+1],[sx(w*.86),y+h*.32],[sx(w*.5),y+h*.48],[sx(w*.38),y+h*.83],[sx(w*.14),y+h*.56]],color);
  p.line(sx(1),y+1,Math.round(sx(w*.36)),Math.round(y+h*.8),C.leafLight);
  return p;
}
function flower(p,x,y,color=C.ivory,small=false) {
  if(small)return p.stamp(x-2,y-2,['  p  ',' pop ','pocop',' pop ','  p  '],{p:color,o:C.gold,c:C.lamp});
  return p.stamp(x-4,y-4,['   ss    ','  spps   ',' ssppsss ','sppggpps','sppgcpps',' ssppss ','  spps   ','   ss    '],{s:C.goldDark,p:color,g:C.gold,c:C.white});
}
function star(p,x,y,color=C.lamp){return p.star(x,y,color);}
function stone(p,x,y,w,h) {
  p.poly([[x+2,y],[x+w-2,y],[x+w,y+2],[x+w-1,y+h],[x,y+h-1],[x,y+2]],C.stoneDeep);
  p.poly([[x+2,y+1],[x+w-3,y+1],[x+w-2,y+3],[x+w-3,y+h-2],[x+1,y+h-2],[x+1,y+3]],C.stone);
  p.line(x+3,y+1,x+w-4,y+1,C.stoneLight).rect(x+2,y+3,2,1,C.stoneLight);
  return p;
}
function lantern(p,x,y,small=false) {
  const w=small?10:17,h=small?22:34;
  p.line(x+w/2|0,y-6,x+w/2|0,y+2,C.shadow,2);
  p.poly([[x-2,y+7],[x+w/2,y],[x+w+2,y+7]],C.shadow);
  p.line(x,y+7,x+w,y+7,C.goldDark);
  p.rect(x+2,y+8,w-3,h-12,C.goldDark).rect(x+4,y+9,w-7,h-14,C.lamp);
  p.line(x+3,y+8,x+3,y+h-5,C.shadow).line(x+w-2,y+8,x+w-2,y+h-5,C.shadow);
  p.poly([[x+w/2-1,y+11],[x+w/2+2,y+15],[x+w/2+1,y+h-8],[x+w/2-1,y+h-7],[x+w/2-3,y+h-11]],C.white);
  p.line(x+1,y+h-5,x+w,y+h-5,C.shadow,2).line(x+4,y+h-2,x+w-3,y+h-2,C.goldDark);
  p.poly([[x+4,y+h-1],[x+w-3,y+h-1],[x+w/2,y+h+4]],C.shadow);
  return p;
}
export async function ground() {
  await plane('01 · Indigo night, stone arch and distant water');
  await paint(new Pen().rect(0,0,275,377,C.night),{layers:STATIC});
  const p=new Pen();
  p.poly([[204,61],[275,33],[275,133],[197,130],[184,113]],C.blue);
  p.rect(194,105,81,27,C.water).line(204,108,274,108,C.reflection);
  for(const [x,y,w,col]of [[215,112,13,C.reflection],[233,116,9,C.goldDark],[246,111,6,C.gold],[207,120,17,C.reflection],[250,120,13,C.reflection],[231,124,10,C.gold],[220,128,7,C.goldDark],[266,127,8,C.reflection]])p.rect(x,y,w,1,col);
  // Narrow village silhouettes remain outside the live song display.
  for(const [x,y,w,h]of [[224,73,10,31],[238,65,13,40],[253,57,11,50],[267,69,9,37]]) {
    p.rect(x,y,w,h,C.shadow).poly([[x-2,y],[x+w/2,y-11],[x+w+2,y]],C.shadow);
    p.rect(x+3,y+7,2,4,C.gold).rect(x+w-4,y+18,2,3,C.lamp);
  }
  p.poly([[212,97],[216,80],[219,97]],C.shadow).poly([[269,67],[272,39],[274,66]],C.shadow);
  // Curved masonry rather than rectangular UI borders.
  p.curve(16,108,-7,15,83,0,C.shadow,31).curve(16,106,-3,17,83,0,C.stoneDeep,27).curve(17,103,1,19,82,1,C.stone,18);
  for(const [x,y,w,h]of [[8,89,22,12],[3,72,22,13],[4,55,23,13],[9,38,23,13],[19,24,23,12],[34,12,23,10],[55,2,22,10]])stone(p,x,y,w,h);
  p.oval(9,18,30,42,'#645548').oval(13,22,22,33,'#8c6e50');
  lantern(p,16,18);
  p.oval(232,2,17,21,C.gold).oval(229,0,15,19,C.night);
  p.rect(239,5,1,2,C.white).line(237,20,241,18,C.ivory);
  for(const [x,y]of [[180,5],[214,13],[260,21],[198,20],[118,17],[151,4]])star(p,x,y,C.gold);
  p.rect(176,16,1,1,C.teal).rect(222,25,1,1,C.teal).rect(261,7,1,1,C.teal);
  await paint(p,{layers:UPPER});
  // Repeating side margins have constant stem cross-sections at both ends.
  for(const[id,w,right]of [['playlist.left.rail',12,false],['playlist.right.rail',20,true]]) {
    const q=new Pen().rect(0,0,w,29,C.night);
    const stem=right?16:4;
    q.line(stem,0,stem,28,C.vine,2);
    leaf(q,stem,1,5,8,right?-1:1);leaf(q,stem,14,5,10,right?-1:1,C.leafLight);
    if(right){q.rect(0,0,5,29,C.blue).line(1,0,1,28,C.water);}
    await sprite(id,q);
  }
  // A stone landing under the sleeping cat, with book spines for the counters.
  const f=new Pen().rect(0,339,275,38,C.blue);
  for(const[x,y,w,h]of [[1,343,37,13],[39,340,37,16],[78,341,45,14],[1,360,33,16],[36,358,46,18],[85,359,48,17],[225,342,49,14],[232,359,42,16]])stone(f,x,y,w,h);
  f.rect(129,344,97,17,C.shadow).rect(132,346,91,13,C.stoneDeep).line(133,346,219,346,C.goldDark);
  f.rect(135,361,91,13,C.shadow).rect(138,363,86,10,C.blue).line(138,363,218,363,C.stone);
  f.line(227,346,227,356,C.goldDark).line(229,350,229,358,C.goldDark);
  await paint(f,{layers:['playlist.bottom.left','playlist.bottom.right']});
  await call('studio_options',{
    playlist_colors:{Normal:C.fur,Current:C.ivory,NormalBG:C.night,SelectedBG:C.stoneDeep,MbFG:C.ivory,MbBG:C.night},
    visualizer_colors:[C.night,C.blue,C.ivory,C.lamp,C.gold,C.teal,C.teal,C.leafLight,C.leaf,C.vine,C.water,C.water,C.blue,C.blue,C.night,C.night,C.night,C.night,C.ivory,C.gold,C.teal,C.leaf,C.water,C.ivory]
  });
}
export async function controlsGround() {
  await plane('02 · Unframed paths and all control states');
  for(const[id,w,h]of [['main.previous',23,18],['main.play',23,18],['main.pause',23,18],['main.stop',23,18],['main.next',22,18],['main.eject',22,16],['main.shuffle',47,15],['main.repeat',28,15],['main.eq.toggle',23,12],['main.playlist.toggle',23,12],['equalizer.on',26,12],['equalizer.auto',32,12],['equalizer.presets',44,12],['main.mono',27,12],['main.stereo',29,12]])await sprite(id,new Pen().rect(0,0,w,h,C.night));
  await sprite('main.position.track',new Pen().rect(0,0,248,10,C.night).curve(2,6,122,9,246,6,C.vine).line(3,6,244,6,C.leaf));
  await sprite('main.volume.track',new Pen().rect(0,0,68,13,C.night).line(2,7,65,7,C.vine).rect(2,7,[1,62],1,C.teal));
  await sprite('main.balance.track',new Pen().rect(0,0,38,13,C.night).line(2,7,35,7,C.vine).rect([2,34],6,2,3,C.teal));
  const stem=new Pen().rect(0,0,14,63,C.night).line(7,1,7,61,C.leaf).line(8,2,8,59,C.vine);
  leaf(stem,7,16,5,9,-1);leaf(stem,8,36,5,9,1,C.leafLight);leaf(stem,7,51,5,8,-1);
  stem.rect(6,[3,56],2,3,C.teal);
  await sprite('equalizer.band0.track',stem);
  await sprite('equalizer.graph',new Pen().rect(0,0,113,19,C.night));
  await sprite('equalizer.preamp.line',new Pen().rect(0,0,113,1,C.vine));
}
export async function foliage() {
  await plane('03 · Ivy, moonflowers and lantern garden');
  const p=new Pen();
  p.curve(3,222,-1,101,20,47,C.vine,3).curve(5,5,54,20,127,7,C.vine,2).curve(4,74,16,96,59,107,C.vine,2);
  for(const [x,y,w,h,s]of [[4,4,8,12,1],[18,0,9,15,1],[37,0,10,11,-1],[48,2,9,13,1],[66,0,10,12,1],[88,0,9,11,1],[107,2,7,10,1],[8,22,7,12,-1],[8,42,9,12,-1],[3,60,9,13,1],[4,80,9,11,1],[18,77,8,11,1],[26,90,9,12,1],[39,94,9,12,1],[9,104,8,12,-1],[8,146,7,12,-1],[4,171,8,13,1],[3,193,8,13,1],[5,209,9,15,1]])leaf(p,x,y,w,h,s);
  for(const [x,y,col,sm]of [[46,8,C.ivory,false],[77,8,C.ivory,true],[102,9,C.ivory,true],[17,63,C.ivory,false],[7,76,C.ivory,true],[41,101,C.ivory,true],[8,162,C.purple,false],[8,197,C.purple,true]])flower(p,x,y,col,sm);
  // Periwinkle bellflowers between the tail and the outer edge.
  p.curve(40,219,32,175,47,166,C.vine,2).curve(43,218,30,183,33,178,C.leaf,2);
  for(const[x,y]of [[43,164],[29,176],[37,190]]) {
    p.poly([[x,y],[x+5,y+1],[x+7,y+9],[x+5,y+8],[x+3,y+11],[x+1,y+8],[x-2,y+10]],C.furDeep);
    p.poly([[x+1,y+2],[x+4,y+2],[x+5,y+8],[x+3,y+7],[x+1,y+9],[x-1,y+8]],C.purple);
    p.line(x+1,y+7,x+2,y+9,C.ivory);
  }
  leaf(p,38,204,11,16,-1);leaf(p,38,211,13,17,1);
  // Right-hand arch stays in the margin beyond the last EQ stem.
  p.poly([[253,223],[252,177],[254,160],[264,149],[275,150],[275,222]],C.stoneDeep);
  p.poly([[259,219],[259,176],[263,163],[270,158],[275,162],[275,219]],C.shadow);
  p.line(254,177,254,219,C.stone,3).line(255,173,260,160,C.stoneLight,2);
  p.oval(257,172,18,35,'#4b4550').oval(260,178,12,24,'#856b53');
  lantern(p,260,178,true);
  p.curve(272,138,242,150,254,194,C.vine,3).curve(271,210,248,219,257,238,C.vine,2);
  for(const [x,y,s]of [[269,133,-1],[270,147,-1],[255,153,-1],[260,163,1],[251,180,1],[258,205,1],[269,215,-1],[255,226,-1],[272,235,-1]])leaf(p,x,y,8,11,s);
  for(const [x,y,sm]of [[266,141,true],[252,159,true],[266,208,true],[251,222,false],[270,239,true]])flower(p,x,y,C.ivory,sm);
  // A wandering garland crosses the EQ/playlist join only through unique title cells.
  p.curve(48,225,108,217,184,229,C.vine,3).curve(96,225,132,247,181,230,C.vine,2);
  for(const[x,y,s]of [[49,223,-1],[66,220,1],[88,222,-1],[110,225,1],[129,232,1],[151,233,1],[171,229,1],[188,225,1],[210,222,1],[231,220,1]])leaf(p,x,y,9,11,s);
  for(const[x,y,c,sm]of [[60,223,C.ivory,false],[91,224,C.ivory,true],[143,237,C.ivory,true],[183,230,C.ivory,false],[222,225,C.ivory,false]])flower(p,x,y,c,sm);
  await paint(p,{layers:[...UPPER,'playlist.title','playlist.top.left','playlist.top.right']});
  // Scrolling margins carry small flowers as part of the same growing vine.
  for(const[id,w,right]of [['playlist.left.rail',12,false],['playlist.right.rail',20,true]]) {
    const q=new Pen();flower(q,right?15:5,24,C.ivory,true);
    await sprite(id,q);
  }
  const f=new Pen();
  f.curve(0,352,6,339,30,341,C.vine,2).curve(224,373,243,336,274,345,C.vine,2);
  for(const[x,y,w,h,s]of [[0,342,12,14,1],[8,354,10,14,-1],[20,342,8,9,1],[237,354,10,12,1],[251,345,10,13,1],[271,342,11,14,-1],[259,363,10,14,1]])leaf(f,x,y,w,h,s);
  flower(f,247,354,C.ivory,false);flower(f,267,343,C.ivory,true);flower(f,12,372,C.ivory,true);
  await paint(f,{layers:['playlist.bottom.left','playlist.bottom.right']});
}
export async function upperCat() {
  await plane('04 · Ivory cat over the joined garden wall');
  const p=new Pen();
  // Soft silhouette and long tail, in solid native-pixel shade clusters.
  p.curve(24,133,-3,155,11,194,C.furDeep,16).curve(24,132,0,155,12,192,C.furShade,13).curve(24,131,3,155,13,190,C.fur,9).curve(22,136,7,159,13,187,C.ivory,5);
  p.oval(8,105,70,35,C.furDeep).oval(10,104,66,33,C.furShade).oval(13,105,57,26,C.fur).oval(19,106,42,17,C.ivory);
  p.poly([[11,118],[18,110],[27,110],[22,115],[21,122],[15,126]],C.furShade);
  p.poly([[36,105],[48,105],[44,109],[38,112],[35,118],[29,121],[31,113]],C.furShade);
  p.poly([[29,128],[39,123],[57,124],[54,131],[44,135],[32,135]],C.furShade);
  // Head is positioned below transport symbols; the two EQ switch cells inherit its fur.
  p.poly([[53,106],[60,94],[70,105],[80,105],[95,99],[91,117],[94,125],[86,135],[64,137],[51,127]],C.furDeep);
  p.poly([[55,108],[60,98],[69,109],[81,108],[91,103],[87,117],[90,125],[84,132],[65,134],[55,125]],C.furShade);
  p.poly([[57,109],[60,102],[68,112],[81,111],[88,107],[85,118],[88,124],[81,130],[67,132],[58,125]],C.ivory);
  p.poly([[58,106],[62,109],[58,114]],C.pink).poly([[84,112],[88,107],[85,118]],C.pink);
  p.poly([[62,113],[70,110],[74,115],[73,120],[68,119]],C.fur);
  p.line(59,119,65,122,C.shadow).line(77,122,83,120,C.shadow);
  p.rect(65,123,2,1,C.furShade).rect(77,124,3,1,C.furShade);
  p.poly([[68,125],[73,125],[71,128]],C.goldDark);
  p.line(71,128,68,130,C.furDeep).line(71,128,75,130,C.furDeep);
  p.line(59,126,49,123,C.fur).line(60,129,49,130,C.fur);
  p.line(85,126,96,123,C.fur).line(84,129,96,130,C.fur);
  p.curve(50,127,41,125,41,134,C.furDeep,11).curve(50,126,42,126,42,133,C.fur,9).curve(49,126,43,127,43,132,C.ivory,5);
  p.curve(82,129,95,131,97,147,C.furDeep,9).curve(82,129,93,132,95,145,C.furShade,7).curve(83,129,92,133,94,143,C.ivory,4);
  p.line(39,133,40,136,C.furDeep).line(43,133,44,136,C.furDeep);
  // Tufts are a handful of hand-placed clusters, not a noise filter.
  for(const[x,y,c]of [[20,104,C.ivory],[29,104,C.fur],[40,105,C.ivory],[49,107,C.ivory],[13,117,C.fur],[17,113,C.ivory],[9,128,C.fur],[12,160,C.ivory],[6,150,C.fur],[12,178,C.ivory],[61,104,C.ivory],[67,111,C.white],[81,110,C.white],[60,127,C.fur]])p.rect(x,y,2,1,c);
  await paint(p,{layers:CAT_TARGETS});
  // The paw is also painted into the graph background, keeping its silhouette whole.
  // The player's live curve remains in front. Small blossoms meet the ivy.
  const q=new Pen();flower(q,106,120,C.ivory,false);leaf(q,100,122,8,11,1);
  await paint(q,{layers:UPPER});
}
export async function footerCat() {
  await plane('05 · Sleeping cat on the stone landing');
  const p=new Pen();
  p.curve(45,362,11,354,19,374,C.furDeep,10).curve(45,361,15,357,21,373,C.furShade,8).curve(44,359,18,358,23,372,C.ivory,4);
  p.oval(29,341,64,30,C.furDeep).oval(30,341,61,27,C.furShade).oval(33,342,54,23,C.fur).oval(36,342,40,15,C.ivory);
  p.poly([[51,342],[60,342],[56,348],[49,354],[44,355],[45,349]],C.furShade);
  p.poly([[72,344],[80,347],[81,353],[74,359],[69,361],[74,353]],C.furDeep);
  p.poly([[76,350],[73,339],[84,345],[97,345],[109,339],[106,353],[110,360],[103,368],[83,370],[74,363]],C.furDeep);
  p.poly([[79,350],[77,343],[84,348],[97,348],[106,343],[103,354],[106,360],[101,365],[84,367],[78,361]],C.furShade);
  p.poly([[81,351],[80,347],[85,351],[98,350],[104,346],[101,355],[104,360],[99,364],[85,365],[81,360]],C.ivory);
  p.poly([[79,345],[85,350],[80,353]],C.pink).poly([[100,350],[104,346],[102,353]],C.pink);
  p.line(80,356,86,359,C.shadow).line(96,359,102,356,C.shadow);
  p.poly([[89,359],[94,359],[92,362]],C.goldDark).line(92,362,90,364,C.furDeep);
  p.line(80,361,72,359,C.fur).line(101,362,114,360,C.fur);
  p.oval(66,365,22,7,C.furDeep).rect(68,365,18,4,C.ivory).line(72,367,72,369,C.furShade).line(77,367,77,369,C.furShade);
  p.oval(97,365,20,7,C.furDeep).rect(99,365,16,4,C.ivory).line(103,367,103,369,C.furShade).line(108,367,108,369,C.furShade);
  p.rect(38,342,3,1,C.white).rect(54,342,2,1,C.white).rect(83,350,2,1,C.white);
  await paint(p,{layers:['playlist.bottom.left','playlist.bottom.right']});
}
export async function details() {
  await plane('06 · Flower handles, warm lettering and quiet controls');
  for(const held of [false,true]) {
    await canvas({active:true,pressed:held});
    const ink=held?C.white:C.lamp,dy=held?1:0;
    for(const name of ['previous','play','pause','stop','next','eject']) {
      const p=new Pen();
      if(name==='play')p.poly([[9,4+dy],[9,12+dy],[16,8+dy]],ink);
      if(name==='previous')p.poly([[9,4+dy],[3,8+dy],[9,12+dy]],ink).poly([[17,4+dy],[11,8+dy],[17,12+dy]],ink);
      if(name==='next')p.poly([[5,4+dy],[11,8+dy],[5,12+dy]],ink).poly([[13,4+dy],[19,8+dy],[13,12+dy]],ink);
      if(name==='pause')p.rect(7,4+dy,3,9,ink).rect(13,4+dy,3,9,ink);
      if(name==='stop')p.rect(7,4+dy,9,9,ink);
      if(name==='eject')p.poly([[6,9+dy],[11,4+dy],[16,9+dy]],ink).line(6,12+dy,16,12+dy,ink);
      await sprite('main.'+name,p,'current');
    }
    for(const[id,w,h]of [['main.position.thumb',29,10],['main.volume.thumb',14,11],['main.balance.thumb',14,11]]) {
      const p=new Pen().rect(0,0,w,h,C.night);
      const x=Math.floor(w/2),y=4+dy;
      if(w===29){p.line(1,6,27,6,C.leaf);leaf(p,x-4,4,6,6,-1);}
      flower(p,x,y,held?C.white:C.ivory,false);
      await sprite(id,p,'current');
    }
    const bud=new Pen().rect(0,0,11,11,C.night);
    bud.stamp(1,0,['    p    ','   pwp   ','  ppwpp  ',' ppwwwpp ','ppwwwwwpp',' ppwwwpp ','  pgggp  ','   ggg   ','    g    '],{p:held?C.gold:C.goldDark,w:held?C.white:C.ivory,g:C.leaf});
    await sprite('equalizer.band0.thumb',bud,'current');
    const scroll=new Pen().rect(0,0,8,18,C.blue).line(1,0,1,17,C.water);
    flower(scroll,4,8,held?C.white:C.lamp,true);
    await sprite('playlist.scroll.thumb',scroll,'current');
    for(const[id,kind]of [['main.options','flower'],['main.minimize','min'],['main.shade','up'],['main.close','x'],['equalizer.close','x']]) {
      const q=new Pen().rect(0,0,9,9,C.night);
      if(kind==='flower')flower(q,4,4,ink,true);
      if(kind==='min')q.line(2,5,6,5,ink);
      if(kind==='up')q.line(2,5,4,3,ink).line(4,3,6,5,ink);
      if(kind==='x')q.line(2,2,6,6,ink).line(2,6,6,2,ink);
      await sprite(id,q,'current');
    }
    const presets=new Pen().text(6,4+dy,'PRESETS',ink,'small',0);
    await sprite('equalizer.presets',presets,'current');
    for(const active of [false,true]) {
      await canvas({active,pressed:held});
      for(const[id,w,word]of [['main.shuffle',47,'SHUF'],['main.repeat',28,'REP'],['main.eq.toggle',23,'EQ'],['main.playlist.toggle',23,'PL']]) {
        const p=new Pen().text(4,5+dy,word,active?ink:C.leaf,'small',0);
        if(active)p.rect(1,6+dy,1,2,C.gold);
        await sprite(id,p,'current');
      }
      // The cat's fur continues through these sprite cells; only letters are added.
      for(const[id,word]of [['equalizer.on','ON'],['equalizer.auto','AUTO']]) {
        const letterInk=id==='equalizer.auto'?(active?ink:C.leaf):(active?C.shadow:C.stone);
        const p=new Pen().text(5,4+dy,word,letterInk,'small',0);
        await sprite(id,p,'current');
      }
      for(const[id,w,word]of [['main.mono',27,'MONO'],['main.stereo',29,'STEREO']])await sprite(id,new Pen().text(0,5,word,active?C.teal:C.leaf,'small',0),'current');
    }
  }
  await canvas({active:true,pressed:false});
  const q=new Pen().text(117,6,'MOON GARDEN',C.lamp,'small',1);
  q.line(118,13,173,13,C.vine);
  q.rect(72,29,2,2,C.ivory).rect(72,35,2,2,C.ivory);
  await paint(q,{layers:UPPER});
  const f=new Pen();
  f.text(17,350,'+',C.lamp).text(45,350,'-',C.shadow).text(111,353,'...',C.gold,'small',0);
  flower(f,89,350,C.ivory,true);
  f.line(237,350,247,350,C.lamp).line(237,353,247,353,C.lamp).line(237,356,247,356,C.lamp);
  f.line(140,365,140,369,C.gold).poly([[146,365],[142,367],[146,369]],C.gold);
  f.poly([[149,365],[154,367],[149,369]],C.gold).line(158,365,158,369,C.gold).line(161,365,161,369,C.gold);
  f.rect(167,365,4,5,C.gold).poly([[176,365],[180,367],[176,369]],C.gold).line(182,365,182,369,C.gold);
  f.poly([[187,367],[190,364],[193,367]],C.gold).line(187,370,193,370,C.gold);
  await paint(f,{layers:['playlist.bottom.left','playlist.bottom.right']});
  for(const[sheet,w,h]of [['text.bmp',155,18],['numbers.bmp',99,13]]) {
    await call('studio_atlas',{sheet});
    const clip=await call('studio_cluster',{rect:[0,0,w,h]});
    const palette=Object.fromEntries(Object.entries(clip.palette).map(([key,color])=>[key,color.toLowerCase()==='#254c67'?C.ivory:C.night]));
    await paint(new Pen().stamp(0,0,clip.rows,palette));
  }
  await canvas({active:true,pressed:false});
  for(let playback=0;playback<3;playback++) {
    await canvas({playback});
    const p=new Pen().rect(0,0,9,9,C.night);
    if(playback===0)p.rect(3,3,4,4,C.teal);
    if(playback===1)p.poly([[2,2],[2,7],[7,4]],C.teal);
    if(playback===2)p.rect(2,2,2,6,C.gold).rect(5,2,2,6,C.gold);
    await sprite('main.status',p,'current');
  }
  await call('studio_cursors',{action:'draw',overwrite:true,style:'chisel'});
}
export async function polish() {
  await canvas({active:true,pressed:false});
  await plane('07 · Foreground ivy, reflected lanterns and a watchful kitten');
  const p=new Pen();
  p.curve(5,66,48,58,101,67,C.vine,2);
  for(const[x,y,w,h,s]of [[6,11,6,10,1],[25,6,7,9,1],[38,6,7,10,-1],[61,9,7,10,1],[81,5,7,10,1],[8,49,7,11,1],[22,61,7,10,-1],[45,61,7,9,1],[68,62,6,9,-1],[91,61,7,10,1]])leaf(p,x,y,w,h,s);
  for(const[x,y,sm]of [[38,64,true],[81,65,true],[29,11,true],[58,6,true]])flower(p,x,y,C.ivory,sm);
  p.poly([[237,108],[246,108],[242,112],[248,115],[239,120],[244,125],[231,130],[225,130],[236,124],[230,119],[239,114]],C.reflection);
  for(const[x,y,w,c]of [[237,109,7,C.gold],[239,112,4,C.lamp],[236,115,8,C.goldDark],[239,118,4,C.gold],[233,122,7,C.goldDark],[231,126,5,C.gold],[227,129,5,C.goldDark]])p.rect(x,y,w,1,c);
  p.curve(258,127,246,135,243,126,C.shadow,3).oval(250,119,8,13,C.shadow);
  p.poly([[247,119],[247,111],[251,115],[256,111],[257,119],[254,122],[250,122]],C.shadow);
  p.rect(249,118,1,1,C.lamp).rect(254,118,1,1,C.lamp);
  p.rect(246,131,16,3,C.stone).line(247,131,258,131,C.stoneLight);
  await paint(p,{layers:UPPER});
}
export async function finish() {
  await canvas({active:true,pressed:false,playback:0,position:10,volume:20,balance:14,scroll:5,eq:[14,23,18,13,9,6,9,13,17,21,24],preview_playlist_height:145,zoom:2,presentation:true});
  await call('studio_targets',{auto:true});
  const target=path.join(ROOT,'assets/skins/Catamp Moon Garden');
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
  if(before.dirty)throw new Error('Save the current Studio work before rebuilding Moon Garden.');
  await call('studio_open',{path:path.join(ROOT,'assets/skins/Catamp Silverplay.wsz')});
  await canvas({zoom:2,preview_playlist_height:145});
  await ground();await controlsGround();await foliage();await upperCat();await footerCat();await details();await polish();await finish();
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)await build();
