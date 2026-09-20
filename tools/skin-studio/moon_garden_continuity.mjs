import { call, canvas, Pen } from './moonlit.mjs';
import { Pencil, paint } from './moon_garden_refine.mjs';

const N='#171e32', D='#253747', M='#3c5660', G='#5b726d', L='#8b9b84';
const BACK=['main.background','main.title','equalizer.background','equalizer.title'];
function cluster(p,x,y,rows,flip=false){p.rows(x,y,rows,{d:D,m:M,g:G,l:L},flip);}
function ivy(p,x,y,flip=false){cluster(p,x,y,['   d    ','  mgd   ',' dmggd  ','dmmllgd ',' dmmggmd','  dmgmd ','   dmd  ','    d   '],flip);}
function star(p,x,y){p.rows(x,y,['  g  ',' mlg ','glhlg',' glm ','  g  '],{g:'#8a806f',m:'#b9a68a',l:'#e4d0ac',h:'#f1dfbb'});}

export async function repairContinuity(project){
  await canvas({active:true,pressed:false});
  await call('studio_layers',{action:'add',name:'07 · Repaired silhouettes and continuous garden edges'});
  // Restore the actual sky, including the moon's lower tip and distant roofs.
  // The text occupies transparent pixels; a 151-by-28 blank was never necessary.
  const sky=new Pencil();
  for(let y=19;y<57;y++)for(let x=168;x<268;x++)sky.dot(x,y,project(x,y,false));
  // Authored low-contrast cloud clusters cross the old clearing boundary.
  for(const [x,y,rows] of [
    [179,26,['       aaa        ','   aaaabbbbaaaa   ','aaabbbbbbbbbbbbaaa','     aaaaaaaaa    ']],
    [215,32,['          aaa           ','     aaaabbbbaaa        ',' aaabbbbbbbbbbbbaaaaa   ','abbbbbbbbbbbbbbbbbbbbaaa','    aaaaaaaaaaaaaaaa    ']],
    [245,25,['   aa      ',' aabbaa    ','abbbbbbaa  ','  aaaabbbba','      aaaa ']],
  ])sky.rows(x,y,rows,{a:'#1b243b',b:'#202b44'});
  sky.path([[196,39],[201,37],[207,38],[212,37]],'#29364e');
  sky.dot(192,22,'#6b7889').dot(201,35,'#68798a').dot(244,37,'#526b80');
  sky.rows(171,44,[' p     p ','phs   shp','phhs shhp',' shhghhs ','  shghs  ','  p g p  '],{s:'#617886',p:'#b3beae',h:'#eee1bd',g:'#bd9568'});
  await paint(sky,'Restore the moonlit sky beneath the song text',{layers:BACK});
  for(const active of [false,true]){
    await canvas({active});
    for(const [id,word,x,w]of [['main.mono','MONO',212,27],['main.stereo','STEREO',239,29]]){
      const q=new Pencil();
      for(let y=0;y<12;y++)for(let xx=0;xx<w;xx++)q.dot(xx,y,project(x+xx,41+y,false));
      await call('studio_draw',{label:'Channel letters over the continuous village',origin:id,states:'current',operations:[...q.ops,...new Pen().text(0,5,word,active?'#bccab3':'#657f79','small',0).ops]});
    }
  }
  await canvas({active:true});

  // A control is still a piece of the painting. Remove the menu's square ground.
  for(const pressed of [false,true]){
    await canvas({pressed});
    const menu=new Pencil();
    for(let y=0;y<9;y++)for(let x=0;x<9;x++)menu.dot(x,y,project(x+6,y+3,false));
    star(menu,2,2+(pressed?1:0));
    await paint(menu,'Flower menu on the original ivy',{origin:'main.options',states:'current'});
    for(const [id,x,y,kind]of [['main.minimize',244,3,'min'],['main.shade',254,3,'up'],['main.close',264,3,'x'],['equalizer.close',264,119,'x']]){
      const q=new Pencil(),ink=pressed?'#fff1d0':'#f4cf8d';
      for(let yy=0;yy<9;yy++)for(let xx=0;xx<9;xx++)q.dot(xx,yy,project(x+xx,y+yy,false));
      if(kind==='min')q.line(2,5,6,5,ink);
      if(kind==='up')q.path([[2,5],[4,3],[6,5]],ink);
      if(kind==='x')q.line(2,2,6,6,ink).line(2,6,6,2,ink);
      await paint(q,'Window signs over the garden',{origin:id,states:'current'});
    }
  }
  await canvas({pressed:false});

  // The arch turns around the spectrum instead of ending at its upper-left corner.
  const arch=new Pencil();
  arch.path([[21,35],[24,36],[28,38],[31,40],[38,40],[42,41]],D);
  cluster(arch,23,35,['  dd     ',' dmmdd   ','dmggmmd  ',' dglgmmd ','  dggmd  ','   dmd   ','    d    ']);
  cluster(arch,33,37,[' d    ','dmgd  ','dmlgd ',' dmggd','  dmd ','   d  ']);
  arch.path([[20,39],[20,45],[19,52],[20,57],[24,61],[29,62]],G);
  cluster(arch,16,46,['  d    ',' dmgd  ',' dmglmd','  dggmd','   dmd ']);
  ivy(arch,20,58);ivy(arch,29,62,true);
  await paint(arch,'Turn the stone ivy around the spectrum corner',{layers:BACK});

  // Rejoin the seek strip to the scene above and below, across the entire track.
  const seek=new Pencil();
  for(let y=0;y<10;y++)for(let x=0;x<248;x++)seek.dot(x,y,project(x+17,y+72,false));
  // A narrow calm path keeps the moving flower readable without a ten-row rectangle.
  for(let x=0;x<248;x++){
    seek.dot(x,5,N).dot(x,6,x<3||x>244?M:G).dot(x,7,N);
  }
  await paint(seek,'Continuous garden across the seek strip',{origin:'main.position.track'});
  const bank=new Pencil();
  bank.path([[15,73],[20,70],[27,71],[34,73],[43,73],[49,76]],M);
  ivy(bank,16,69,true);ivy(bank,27,70);ivy(bank,39,73,true);
  // Write both the ground and fixed track, so the boundary cannot hide the repair.
  await paint(bank,'Leaves cross the seek ownership boundary',{layers:['main.background','main.position.track']});

  // Round the loose tail before it reaches the shared fader cell. Preserve its fur.
  const tail=new Pencil();
  const edge=[21,21,21,21,20,20,20,19,19,19,19,19,18,18,18,18,18,18,18,18,17,17,17,16,16,15,14,13,12];
  for(let i=0;i<edge.length;i++){
    const y=149+i,r=edge[i];
    for(let x=r+1;x<=24;x++)tail.dot(x,y,N);
    tail.dot(r,y,'#727995').dot(r-1,y,'#a5a5b9');
    if(i<24)tail.dot(r-2,y,'#cbc2c2');
  }
  tail.path([[20,156],[23,160],[23,166],[20,171],[19,180],[23,187]],D);
  cluster(tail,17,168,['    d','   dm','  dmg',' dmmg','dmmgg',' dmgg','  dmg','   dm']);
  cluster(tail,15,181,['  d    ',' dmgd  ','dmmggd ',' dmllgd','  dggmd','   dmd ']);
  await paint(tail,'Rounded tail tip and shaded ivy beside the preamp',{layers:BACK});

  // An irregular, shadowed fern bank replaces the last fader's ruler-straight cut.
  const right=new Pencil();
  const contour=[4,5,5,6,7,8,7,6,5,4,3,3,2,2,1,1,2,3,4,3,2,2,1,1,2,3,4,4,5,4,3,3,2,2,2,3,4,5,4,4,3,2,2,2,3,3,4,5,5,4,3,2,1,1,2,3,3,2,2,1,0,0,0];
  for(let j=0;j<63;j++)for(let x=254;x<254+contour[j];x++)right.dot(x,154+j,N);
  right.path([[262,157],[258,162],[259,168],[255,174],[257,181],[255,188],[258,196],[257,204],[252,216]],D);
  for(const [x,y,flip]of [[254,157,true],[255,187,false],[252,199,true],[251,209,false]])ivy(right,x,y,flip);
  // Upper fronds end naturally above the shared fader fields.
  right.path([[235,151],[240,150],[245,152],[250,151],[255,154]],M);
  cluster(right,236,147,['   d    ','  dmgd  ',' dmggmd ','dmmllgmd',' dmggmd ','  dmd   ']);
  cluster(right,249,150,['   d    ','  dmd   ',' dmggd  ','dmmlgmd ',' dggmd  ','  dmd   ']);
  await paint(right,'Shadowed fern silhouettes around the lantern and last flower',{layers:BACK});
  const lantern=new Pencil();
  // Finish the whole lantern silhouette in its nine-pixel bay, clear of the fader.
  lantern.rows(254,165,[
    '    d    ','    m    ','   dmd   ','  dgmgd  ',' dgggggd ',
    '  dmmd   ','  dlld   ','  dlhd   ','  dlhd   ','  dlld   ',
    '  dgld   ','  dgmd   ',' dmmmmmd ','  dgggd  ','   dmd   ','    d    ',
  ],{d:'#29313f',m:'#927655',g:'#bf9c67',l:'#efcc93',h:'#f4e2be'});
  await paint(lantern,'Complete hanging lantern beside the flower bed',{layers:BACK});

  // A new native twelve-pixel ivy rail; the old one was horizontally squeezed.
  const rail=new Pencil().box(0,0,12,29,N);
  for(let y=0;y<29;y++)rail.dot(2,y,'#233247').dot(3,y,D).dot(4,y,M).dot(5,y,D);
  cluster(rail,2,2,['    d   ','   dmd  ','  dmgmd ',' dmglmd ','dmmggmd ',' dmggd  ','  dmd   ','   d    ']);
  cluster(rail,0,15,[' d      ','dmd     ','dggmd   ','dmglgmd ',' dmggmd ','  dmgd  ','   dd   ']);
  rail.dot(4,12,G).dot(3,24,M).dot(5,27,G);
  await paint(rail,'Native ivy on the repeating playlist rail',{origin:'playlist.left.rail'});
  // Continue the same stem through the unique cap and into the foreground garden.
  const joins=new Pencil();
  for(let y=243;y<252;y++)for(let x=0;x<12;x++){
    const j=y-243;
    if(x<6||j>5)joins.dot(x,y,rail.ink.get(x+','+((y-252+29)%29))||N);
  }
  joins.path([[4,238],[3,242],[4,246],[4,251]],M);
  ivy(joins,0,241,true);
  for(let y=339;y<350;y++)for(let x=0;x<12;x++)joins.dot(x,y,rail.ink.get(x+','+((y-339)%29))||N);
  joins.path([[4,339],[4,346],[7,351],[13,355]],G);
  ivy(joins,0,346);ivy(joins,6,350,true);
  await paint(joins,'Continuous ivy at both ends of the playlist rail',{layers:['playlist.top.left','playlist.bottom.left']});
  await canvas({active:true,pressed:false});
}
