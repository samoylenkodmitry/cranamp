import { call, canvas } from './moonlit.mjs';
import { C } from './moon_garden.mjs';

// Every mark emitted by this pass is a native, opaque one-pixel pencil operation.
// Small cluster charts are authored here; none are lifted from an image.
export class Pencil {
  constructor(){this.ink=new Map();}
  dot(x,y,color){this.ink.set(Math.round(x)+','+Math.round(y),color);return this;}
  box(x,y,w,h,color){for(let j=0;j<h;j++)for(let i=0;i<w;i++)this.dot(x+i,y+j,color);return this;}
  line(x,y,x2,y2,color){
    x=Math.round(x);y=Math.round(y);x2=Math.round(x2);y2=Math.round(y2);
    const dx=Math.abs(x2-x),dy=-Math.abs(y2-y),sx=x<x2?1:-1,sy=y<y2?1:-1;
    let error=dx+dy;
    for(;;){this.dot(x,y,color);if(x===x2&&y===y2)break;const e=error*2;if(e>=dy){error+=dy;x+=sx;}if(e<=dx){error+=dx;y+=sy;}}
    return this;
  }
  path(points,color){for(let i=1;i<points.length;i++)this.line(...points[i-1],...points[i],color);return this;}
  rows(x,y,rows,palette,flip=false){
    rows.forEach((row,j)=>[...row].forEach((v,i)=>{if(palette[v])this.dot(x+(flip?row.length-1-i:i),y+j,palette[v]);}));
    return this;
  }
  get ops(){return [...this.ink].map(([xy,color])=>{const [x,y]=xy.split(',').map(Number);return {op:'pixel',x,y,color,brush_size:1};});}
}
export async function paint(p,label,args={}){
  const ops=p.ops;
  for(let i=0;i<ops.length;i+=5000)await call('studio_draw',{label,operations:ops.slice(i,i+5000),states:'all',...args});
}
const SKY='#171e32', DEEP='#253747', MOSS='#3c5660', GREEN='#5b726d', LIGHT='#8b9b84';
const LEAF=['   ss   ','  shms  ',' smhhms ','smmmhhms',' smmmhs ','  smhs  ','   ss   '];
function leaf(p,x,y,flip=false,lit=false){
  p.rows(x,y,LEAF,{s:DEEP,m:lit?GREEN:MOSS,h:lit?LIGHT:GREEN},flip);
}
function sprig(p,points,leaves){
  p.path(points,MOSS);
  for(const[x,y,flip,lit]of leaves)leaf(p,x,y,flip,lit);
}
function flower(p,x,y,blue=false){
  p.rows(x-4,y-4,[
    '   ss    ','  smms   ',' ssmhss  ','smmhhmms','smhgghms',' smhhms ',
    '  smms  ','   ss   ',
  ],{s:blue?'#414d78':'#8b765e',m:blue?'#858eb2':'#d0bb9b',h:blue?'#b0b9d3':'#f0dfbd',g:blue?'#d8c69f':'#e2b976'});
}
function tiny(p,x,y,blue=false){
  p.rows(x-2,y-2,['  h  ',' mhm ','hhghh',' mhm ','  h  '],{m:blue?'#687da3':'#b29b7c',h:blue?'#919cbb':'#dcc7a3',g:'#f0dfa6'});
}
function bell(p,x,y,flip=false){
  p.rows(x,y,['   ss   ','  spms  ','  sphms ',' sppmhs ',' spphhms','sppphhms','sphhhhs ',' hh hh  ',' h   h  '],
    {s:'#394961',p:'#697aa2',m:'#93a0c0',h:'#d7d5d3'},flip);
}
function moth(p,x,y){
  p.rows(x,y,[' p     p ','phs   shp','phhs shhp',' shhghhs ','  shghs  ','  p g p  '],
    {s:'#617886',p:'#b3beae',h:'#eee1bd',g:'#bd9568'});
}
const BACK=['main.background','main.title','equalizer.background','equalizer.title'];
const FOOT=['playlist.bottom.left','playlist.bottom.right'];
export async function refine(project){
  await canvas({active:true,pressed:false});
  await call('studio_layers',{action:'add',name:'05 · Pixel foliage, masonry and proportional details'});
  const p=new Pencil();
  // Shape the arch around the fixed spectrum footprint, eliminating its chopped flower.
  for(let y=39;y<67;y++){
    const edge=y<45?25-Math.floor((y-39)/2):y<60?22:22+(y-59)*2;
    p.box(edge,y,107-edge,1,SKY);
  }
  p.path([[23,36],[22,40],[19,45],[18,53],[20,59],[27,65]],MOSS);
  leaf(p,14,42,true,false);leaf(p,14,50,false,true);leaf(p,16,57,true,false);
  // The live spectrum stays quiet. A growing branch gives its lower edge a natural ending.
  sprig(p,[[19,66],[30,65],[44,67],[58,67],[74,65],[91,67],[103,64]],[
    [24,61,false,true],[35,65,true,false],[45,61,true,false],[56,66,false,true],
    [69,60,false,false],[81,65,true,false],[95,61,true,true],
  ]);
  tiny(p,37,63);tiny(p,78,66,true);moth(p,91,61);
  // Sparse pendant ivy connects the upper arch to the display without covering glyphs.
  sprig(p,[[95,12],[99,17],[105,20],[107,26],[107,34]],[[98,15,false,false],[103,22,true,false],[104,31,false,false]]);
  p.dot(111,20,'#6d808b').dot(113,18,'#a7aea0');
  // Three quiet constellations inhabit otherwise empty sky around the readouts.
  p.path([[180,39],[185,36],[192,38]],'#273247');
  p.dot(180,39,'#9caaab').dot(185,36,'#b9bdad').dot(192,38,'#7b929a');
  p.rows(187,46,['  h  ','  m  ','hmhmh','  m  ','  h  '],{h:'#485c70',m:'#b6b498'});
  p.dot(199,43,'#708798').dot(207,38,'#b4baa9');
  // Gentle water reflections replace the empty band below the volume vines.
  for(const[x,y,w,c]of [[112,70,12,DEEP],[132,71,7,MOSS],[159,71,5,DEEP],[179,71,12,DEEP],[202,71,5,MOSS],[197,83,8,C.water],[217,84,7,C.reflection],[232,82,4,C.goldDark],[251,84,11,C.water],[174,85,7,DEEP]])p.box(x,y,w,1,c);
  // Plants occupy the available 43-pixel bay between the preamp and ten EQ bands.
  sprig(p,[[49,219],[48,209],[51,196],[49,183],[54,174],[59,172],[63,175]],[
    [42,208,true,true],[48,200,false,false],[43,190,true,false],[50,181,false,true],
  ]);
  sprig(p,[[56,220],[59,207],[64,199],[65,190],[69,186],[73,188]],[
    [54,211,true,false],[60,203,false,true],[63,195,true,false],
  ]);
  sprig(p,[[45,219],[42,211],[40,199],[42,194],[46,193]],[[37,202,true,false]]);
  bell(p,60,174);bell(p,68,187,true);bell(p,42,194);
  flower(p,54,213,true);tiny(p,39,215);tiny(p,70,216,true);
  moth(p,50,158);
  // Night grass breaks the ruler-straight lower edge of the flower beds.
  for(const[x,y,flip,lit]of [[81,217,false,false],[93,219,true,true],[105,218,false,false],
    [121,217,true,false],[139,219,false,true],[153,217,true,false],[174,219,false,false],
    [191,218,true,true],[211,217,false,false],[230,218,true,false],[245,216,false,true]]){
    leaf(p,x,y,flip,lit);p.line(x+3,y+4,x+4,y-2,DEEP);
  }
  tiny(p,114,220,true);tiny(p,186,222);tiny(p,238,219,true);
  // Tiny fireflies sit in the four-pixel gaps, clear of the moving flower heads.
  for(const[x,y]of [[93,174],[111,194],[129,164],[147,181],[165,207],[183,173],[201,192],[219,161],[237,199]]){
    p.dot(x,y,C.goldDark).dot(x+1,y,C.lamp).dot(x,y+1,DEEP);
  }
  await paint(p,'Individual ivy, bells, moths and fireflies',{layers:BACK});

  // The shared EQ strip gets a matching root cluster at its foot in all 28 frames.
  const roots=new Pencil();
  roots.path([[0,62],[2,58],[3,60],[5,61],[7,58],[9,61],[11,59],[13,62]],DEEP);
  roots.dot(2,59,MOSS).dot(3,61,GREEN).dot(10,61,MOSS).dot(11,60,GREEN);
  roots.rows(0,18,[' h     ',' mh    ',' mmh   ',' smmh  ',' smmmh ','  smmmh','   smmh','    smh','     sh'],{s:DEEP,m:MOSS,h:GREEN});
  roots.rows(8,37,['     h','    hm','   hmm','  hmms',' hmmm ','hmmms ','hmms  ','hms   ','hs    '],{s:DEEP,m:GREEN,h:LIGHT});
  await paint(roots,'Roots continue out of every live EQ stem',{origin:'equalizer.band0.track'});

  // Keep the reference wall's irregular clusters in the repeated title cell.
  // Each sample retains native proportions; no long foliage strip is stretched.
  const title=new Pencil();
  for(let y=0;y<20;y++)for(let x=0;x<25;x++)title.dot(x,y,project(159+x,232+y));
  await paint(title,'Seamless native ivy across the playlist join',{origin:'playlist.top.tile'});
  // Unique middle keeps the vine from looking like a repeated wallpaper motif.
  const center=new Pencil();
  for(let y=0;y<20;y++)for(let x=0;x<100;x++){
    // Four columns transition between the repeated edge and unique middle.
    const px=(x+87)%25,edge=Math.min(x,99-x);
    const tiled=title.ink.get(px+','+y)||SKY;
    center.dot(x,y,edge<2?tiled:project(x+87,232+y));
  }
  tiny(center,23,11);tiny(center,74,9,true);
  sprig(center,[[37,10],[43,11],[48,15],[53,15]],[[41,10,false,false],[47,12,true,false]]);
  await paint(center,'Distinct moonflowers across the central join',{origin:'playlist.title'});

  // Books have native proportions, readable spines and stepped page ends.
  const f=new Pencil();
  f.box(119,339,156,38,SKY);
  for(const[x,y,w]of [[119,341,23],[144,340,35],[179,341,48],[226,340,48]]){
    f.box(x,y,w,2,'#29384d');f.box(x+2,y,w-5,1,'#45546a');
  }
  f.box(126,343,102,19,'#101a2c').box(129,345,96,15,C.stoneDeep);
  f.box(126,342,104,2,'#655d56').box(129,344,96,1,'#9c8b6b');
  f.line(127,345,127,359,'#756951').line(129,361,225,361,'#19283d');
  f.box(130,347,2,11,'#99866b').box(222,347,2,11,'#7d7565');
  f.path([[226,344],[260,341],[266,344],[263,356],[227,360]],'#829185');
  for(let y=345;y<357;y++)f.box(229,y,34-Math.floor((y-345)/4),1,['#979681','#a9a18a','#b9ac91'][y%3]);
  for(let y=347;y<357;y+=3)f.line(232,y,259-(y-345)/3,y-1,'#727c76');
  f.path([[228,346],[228,357],[260,354]],'#56685e');
  f.box(133,362,94,14,'#101a2c').box(136,364,90,11,C.blue);
  f.line(133,362,225,362,'#788077').line(135,363,224,363,'#a99470');
  f.line(134,365,134,374,'#667a79').line(136,375,224,375,'#536370');
  f.path([[228,362],[266,358],[271,361],[268,372],[228,376]],'#506777');
  for(let y=363;y<375;y++)f.box(229,y,38-Math.floor((y-363)/4),1,['#788982','#87978b','#9a9e8c'][y%3]);
  for(let y=365;y<375;y+=3)f.line(232,y,264-(y-363)/4,y-2,'#596f6a');
  // Small vegetation grows around the book stack instead of becoming a frame.
  sprig(f,[[269,376],[266,368],[269,361],[271,351],[273,340]],[
    [264,368,true,false],[266,358,false,true],[267,345,true,false],
  ]);
  flower(f,264,374);tiny(f,122,371,true);leaf(f,113,371,true,false);
  // The footer icons stay inside their classic click bounds.
  const marks=[
    [18,350,['  x  ','  x  ','xxxxx','  x  ','  x  ']],
    [47,351,['xxxxx']],
    [77,350,['    x','x  x ',' xx  ']],
    [112,352,['x x x']],
    [238,350,['xxxxxxxxx','         ','xxxxxxxxx','         ','xxxxxxxxx']],
  ];
  for(const[x,y,rows]of marks)f.rows(x,y,rows,{x:x>=228?'#38434b':x>=35&&x<101?'#4c536b':C.ivory});
  f.line(140,365,140,369,C.gold).rows(142,365,['   x','  xx',' xxx','  xx','   x'],{x:C.gold});
  f.rows(149,365,['x    ','xxx  ','xxxxx','xxx  ','x    '],{x:C.gold});
  f.line(158,365,158,369,C.gold).line(161,365,161,369,C.gold).box(167,365,4,5,C.gold);
  f.rows(176,365,['x    ','xxx  ','xxxxx','xxx  ','x    '],{x:C.gold}).line(182,365,182,369,C.gold);
  f.rows(187,365,['   x   ','  xxx  ',' xxxxx ','       ','xxxxxxx'],{x:C.gold});
  await paint(f,'Native book spines, pages and footer plants',{layers:FOOT});

  // Smaller ink signs sit on the actual fur/ivy pixels and move one pixel when pressed.
  const glyphs={
    previous:['   x   x','  xx  xx',' xxx xxx','xxxx xxxx',' xxx xxx','  xx  xx','   x   x'],
    play:['x      ','xx     ','xxxx   ','xxxxxx ','xxxx   ','xx     ','x      '],
    pause:['xx  xx','xx  xx','xx  xx','xx  xx','xx  xx','xx  xx','xx  xx'],
    stop:['xxxxx','xxxxx','xxxxx','xxxxx','xxxxx'],
    next:['x   x   ','xx  xx  ','xxx xxx ','xxxx xxxx','xxx xxx ','xx  xx  ','x   x   '],
    eject:['   x   ','  xxx  ',' xxxxx ','xxxxxxx','       ','xxxxxxx'],
  };
  const slots=[['previous',16,88,23,18],['play',39,88,23,18],['pause',62,88,23,18],['stop',85,88,23,18],['next',108,88,22,18],['eject',136,89,22,16]];
  for(const pressed of [false,true]){
    await canvas({active:true,pressed});
    for(const[name,x,y,w,h]of slots){
      const q=new Pencil();
      for(let yy=0;yy<h;yy++)for(let xx=0;xx<w;xx++)q.dot(xx,yy,project(x+xx,y+yy));
      const rows=glyphs[name],ox=Math.floor((w-rows.reduce((a,r)=>Math.max(a,r.length),0))/2),oy=5+(pressed?1:0);
      rows.forEach((row,j)=>[...row].forEach((v,i)=>{
        if(v!=='x')return;
        const color=project(x+ox+i,y+oy+j);
        const rgb=[1,3,5].map(k=>parseInt(color.slice(k,k+2),16));
        q.dot(ox+i,oy+j,rgb[0]*.3+rgb[1]*.59+rgb[2]*.11>145?'#364055':pressed?C.white:C.lamp);
      }));
      await paint(q,'Fine transport ink on the continuous illustration',{origin:'main.'+name,states:'current'});
    }
  }
  await canvas({active:true,pressed:false});
}

export async function fillMissedSpace(){
  await canvas({active:true,pressed:false});
  // Ask the exact mapping, rather than inferring an exclusion from a broad readout box.
  for(const rect of [[40,24,70,43],[110,35,159,23],[17,72,248,10],[0,339,34,22],[116,339,20,38]]){
    const coverage=await call('studio_coverage',{rect});
    if(!coverage.counts.paintable)throw new Error('Expected paintable source at '+rect);
  }
  await call('studio_layers',{action:'add',name:'06 · Reclaimed garden margins and moonlit details'});
  const p=new Pencil();
  // A fern climbs the narrow land between the clock and its changing readouts.
  sprig(p,[[103,63],[105,56],[102,49],[105,42],[105,34],[109,30]],[
    [99,55,true,false],[101,47,false,true],[99,39,true,false],[104,32,false,false],
  ]);
  tiny(p,106,52,true);
  // Real space around the digit sprites: a quiet edge of leaves, without altering digits.
  sprig(p,[[41,37],[42,31],[40,26],[44,22]],[[37,28,false,false],[40,21,false,true]]);
  p.dot(44,40,MOSS).dot(45,41,GREEN).dot(102,28,LIGHT);
  // Ivy below the song line wraps around, rather than broadly avoiding, bitrate and rate.
  sprig(p,[[115,38],[127,38],[134,41],[143,40],[151,36],[163,37],[174,40]],[
    [125,35,false,false],[135,37,true,false],[146,34,false,false],[167,36,true,false],
  ]);
  sprig(p,[[130,53],[137,50],[144,53],[150,52],[152,48]],[
    [130,48,true,false],[140,50,false,true],[145,45,false,false],
  ]);
  flower(p,140,46,true);
  sprig(p,[[173,54],[184,52],[194,55],[205,53],[214,55]],[
    [176,49,true,false],[187,51,false,true],[202,48,false,false],
  ]);
  tiny(p,197,53);moth(p,173,44);
  // Small constellations and trailing foliage occupy the real margins of the song display.
  sprig(p,[[213,19],[223,22],[235,22],[248,24],[262,22],[268,28],[265,37]],[
    [223,18,false,false],[246,20,true,false],[263,25,false,false],[262,33,true,false],
  ]);
  tiny(p,240,22,true);
  // These low-contrast water reeds are deliberately stored beneath the spectrum.
  // Coverage marks them O: visible in the editor; the live player fills that exact footprint.
  p.path([[27,57],[35,54],[45,55],[57,51],[68,52],[79,49],[91,50],[99,48]],DEEP);
  for(const[x,y]of [[29,55],[44,52],[58,50],[74,48],[87,47]]){
    p.rows(x,y,[' s ','msm',' sm','  s'],{s:DEEP,m:MOSS});
  }
  p.box(28,59,13,1,DEEP).box(49,59,18,1,DEEP).box(77,59,16,1,DEEP);
  // Keep the exact live text footprints calm. Everything surrounding them remains drawable.
  const text=[[48,26,9,13],[60,26,9,13],[78,26,9,13],[90,26,9,13],
    [111,27,150,8],[111,41,18,8],[156,41,12,8],[212,41,27,12],[239,41,29,12]];
  for(const key of [...p.ink.keys()]){
    const[x,y]=key.split(',').map(Number);
    if(text.some(([a,b,w,h])=>x>=a&&x<a+w&&y>=b&&y<b+h))p.ink.delete(key);
  }
  await paint(p,'Pixel ferns, moths and flowers in the missed readout margins',{layers:BACK});
  // Tiny leaves fill the seek track's empty native rows in every state.
  const seek=new Pencil();
  for(const x of [2,23,47,72,96,121,146,171,195,219,241]){
    seek.rows(x,0,[' s  ','shs ',' shs'],{s:DEEP,h:MOSS});
    seek.dot(x+1,8,MOSS).dot(x+2,9,GREEN);
  }
  seek.line(0,5,2,5,GREEN).line(245,5,247,5,GREEN);
  await paint(seek,'Fine native leaves in the seek margins',{origin:'main.position.track'});
  const f=new Pencil();
  sprig(f,[[3,339],[8,342],[15,342],[23,346],[29,345]],[
    [0,338,false,false],[6,339,true,true],[16,341,false,false],[23,343,true,false],
  ]);
  tiny(f,10,345);tiny(f,29,347,true);
  sprig(f,[[120,340],[119,348],[122,354],[120,361],[122,369],[125,376]],[
    [116,342,false,false],[119,350,true,true],[115,357,false,false],
    [120,365,false,true],[122,372,true,false],
  ]);
  tiny(f,123,346,true);tiny(f,120,369);
  await paint(f,'Garden grows through the empty footer margins',{layers:FOOT});
  await call('studio_cursors',{action:'draw',style:'paw',overwrite:true});
  await canvas({active:true,pressed:false});
}
