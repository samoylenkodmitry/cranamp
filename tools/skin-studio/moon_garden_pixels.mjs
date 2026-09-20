import fs from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { ROOT, call, canvas } from './moonlit.mjs';
import { C, controlsGround, details } from './moon_garden.mjs';
import { refine, fillMissedSpace } from './moon_garden_refine.mjs';
import { repairContinuity } from './moon_garden_continuity.mjs';

const NIGHT='#171e32';
C.night=NIGHT;
const STATIC=['main.background','main.title','equalizer.background','equalizer.title','playlist.top.left','playlist.top.tile','playlist.title','playlist.top.right','playlist.left.rail','playlist.right.rail','playlist.bottom.left','playlist.bottom.right'];
const FIXED=['main.previous','main.play','main.pause','main.stop','main.next','main.eject','main.shuffle','main.repeat','main.eq.toggle','main.playlist.toggle','main.mono','main.stereo','main.position.track','equalizer.on','equalizer.auto','equalizer.presets','equalizer.graph'];
let grid;
const clamp=(n,a,b)=>Math.max(a,Math.min(b,n));
const smooth=t=>{t=clamp(t,0,1);return t*t*(3-2*t);};
const sourceUi=[
  [67,37,40,13],[67,51,39,15],[118,34,73,12],[115,49,99,8],
  [117,64,68,11],[110,78,79,14],[57,138,160,18],[58,157,157,62],
];
function sample(x,y,clean=true){
  x=clamp(Math.round(x),0,274);y=clamp(Math.round(y),0,376);
  if(clean&&sourceUi.some(([a,b,w,h])=>x>=a&&x<a+w&&y>=b&&y<b+h))return NIGHT;
  return grid.palette[parseInt(grid.rows[y].slice(x*2,x*2+2),16)];
}
export function project(x,y,quiet=true){
  if(y>=252&&y<339){
    if(x<12)return sample(x*2.2,253+(y-252)%29);
    if(x>=255)return x<260?C.blue:sample(244+(x-260)*2,253+(y-252)%29);
    return NIGHT;
  }
  if(y>=339){
    if(x>=129&&x<226&&y>=345&&y<361)return C.stoneDeep;
    if(x>=136&&x<226&&y>=362&&y<374)return C.blue;
    if(x>=119)return NIGHT;
    // One uniform scale keeps the sleeping cat's head, body and paws proportional.
    return sample(98+(x-16)*1.35,303+(y-339)*1.35,false);
  }
  let sx=x,sy=y;
  const side=1-smooth((x-110)/30);
  // Translate the upper cat as one shape. Transition only in the surrounding garden.
  const shift=smooth((y-58)/17)*(1-smooth((y-187)/34))*side;
  sy-=17*shift;
  sx+=6*side*(1-smooth((y-220)/28));
  // Curve the loose tail into the margin instead of letting the shared preamp cell cut it.
  sx+=8*smooth((y-129)/24)*(1-smooth((y-189)/24))*(1-smooth((x-34)/20));
  // Lift the lantern into the margin above the live spectrum without squeezing it.
  if(x<43&&y<68)sy+=8*(1-smooth((y-44)/24));
  if(x>208&&y<50)sy+=10*smooth((x-208)/14)*(1-smooth((y-25)/25));
  let color=sample(sx,sy);
  // Native runtime regions need quiet ink behind their changing content.
  if(quiet&&((x>=43&&x<102&&y>=24&&y<41)||(x>=110&&x<261&&y>=25&&y<53)))color=NIGHT;
  if(quiet&&x>=16&&x<265&&y>=72&&y<82)color=NIGHT;
  if(quiet&&y>=155&&y<218&&((x>=21&&x<35)||(x>=78&&x<254)))color=NIGHT;
  return color;
}
async function pixels(name,ops,args={}){
  for(let start=0;start<ops.length;start+=5000)await call('studio_draw',{label:name,operations:ops.slice(start,start+5000),states:'all',...args});
}
const dot=(x,y,color)=>({op:'pixel',x,y,color,brush_size:1});

export async function loadReference(){
  grid=JSON.parse(await fs.readFile(path.join(ROOT,'assets/skins/sources/Moon Garden reference pixels.json'),'utf8'));
}
export async function paintReference(){
  await loadReference();
  await call('studio_layers',{action:'add',name:'02 · Continuous reference colors — one-pixel pencil'});
  const ops=[];
  for(let y=0;y<377;y++)for(let x=0;x<275;x++){
    if(y>=252&&y<339&&x>=12&&x<255)continue;
    ops.push(dot(x,y,project(x,y)));
  }
  await pixels('Paint the joined reference pixel by pixel',ops,{layers:[...STATIC.filter(id=>id!=='playlist.top.tile'),...FIXED]});
  const topTile=[];
  for(let y=0;y<20;y++)for(let x=0;x<25;x++)topTile.push(dot(x,y,sample(159+x,232+y,false)));
  await pixels('Continuous garden on the shared title tile',topTile,{origin:'playlist.top.tile'});
  console.log({reference_pencil_marks:ops.length,palette:grid.palette.length});
}
function displayInk(color){
  const rgb=[1,3,5].map(i=>parseInt(color.slice(i,i+2),16));
  return rgb[0]*.3+rgb[1]*.59+rgb[2]*.11>137?C.shadow:C.ivory;
}
export async function nativeCorrections(){
  await canvas({active:true,pressed:false});
  await call('studio_layers',{action:'add',name:'04 · Native pixel repairs and readable controls'});
  // Restore the single-pixel seek stem after cleaning the conceptual UI.
  await pixels('Seek stem',Array.from({length:242},(_,i)=>dot(i+3,6,C.leaf)),{origin:'main.position.track'});
  const glyph={
    O:['111','101','101','101','111'],N:['1001','1101','1011','1001','1001'],
    A:['010','101','111','101','101'],U:['101','101','101','101','111'],T:['111','010','010','010','010'],
  };
  for(const active of [false,true])for(const pressed of [false,true]){
    await canvas({active,pressed});
    for(const[id,word,x,y,w]of [['equalizer.on','ON',16,134,26],['equalizer.auto','AUTO',42,134,32]]){
    const ops=[];
    for(let yy=0;yy<12;yy++)for(let xx=0;xx<w;xx++)ops.push(dot(xx,yy,project(x+xx,y+yy)));
    let offset=5;
    for(const letter of word){
      const rows=glyph[letter];
      rows.forEach((row,dy)=>[...row].forEach((bit,dx)=>{
        if(bit==='1'){
          const yy=4+dy+(pressed?1:0),ink=displayInk(project(x+offset+dx,y+yy));
          ops.push(dot(offset+dx,yy,active?ink:ink===C.shadow?C.stone:C.leafLight));
        }
      }));
      offset+=rows[0].length+1;
    }
    if(active)ops.push(dot(2,6+(pressed?1:0),C.lamp));
    await pixels('Readable letters on the cat fur',ops,{origin:id,states:'current'});
    }
  }
  await canvas({active:true,pressed:false});
  // The footer uses the cat's coat as its action area; use ink with local contrast.
  const footer=[];
  const marks=[
    [19,352,['010','111','010']],
    [48,353,['111']],
    [77,352,['001','101','010']],
    [111,353,['10101']],
    [239,351,['111111111','000000000','111111111','000000000','111111111']],
  ];
  for(const[x,y,rows]of marks)rows.forEach((row,dy)=>[...row].forEach((bit,dx)=>{if(bit==='1')footer.push(dot(x+dx,y+dy,displayInk(project(x+dx,y+dy))));}));
  await pixels('Footer ink',footer,{layers:['playlist.bottom.left','playlist.bottom.right']});
  await call('studio_options',{
    playlist_colors:{Normal:C.fur,Current:C.ivory,NormalBG:NIGHT,SelectedBG:C.stoneDeep,MbFG:C.ivory,MbBG:NIGHT},
    visualizer_colors:[NIGHT,C.blue,C.ivory,C.lamp,C.gold,C.teal,C.teal,C.leafLight,C.leaf,C.vine,C.water,C.water,C.blue,C.blue,NIGHT,NIGHT,NIGHT,NIGHT,C.ivory,C.gold,C.teal,C.leaf,C.water,C.ivory],
  });
}
export async function finish(){
  const names=['01 · Native control frames','02 · Reference scene in native pixels','03 · Text, flower handles and playback states','04 · Pixel repairs and readable controls','05 · Pixel foliage, masonry and proportional details','06 · Reclaimed garden margins and moonlit details','07 · Repaired silhouettes and continuous garden edges'];
  const {layers}=await call('studio_layers',{action:'list'});
  for(const [index,layer]of layers.entries())await call('studio_layers',{action:'set',id:layer.id,name:names[index]});
  await canvas({active:true,pressed:false,playback:0,position:10,volume:20,balance:14,scroll:5,eq:[14,23,18,13,9,6,9,13,17,21,24],preview_playlist_height:145,zoom:2,presentation:true});
  await call('studio_targets',{auto:true});
  const out=path.join(ROOT,'assets/skins/Catamp Moon Garden');
  const validation=await call('studio_validate');
  if(validation.divergences?.length)throw new Error(JSON.stringify(validation));
  const exported=await call('studio_export',{path:out+'.wsz'});
  if(exported.hard_to_read?.length)throw new Error(JSON.stringify(exported.hard_to_read));
  console.log(exported);
  console.log(await call('studio_project',{action:'save',path:out+'.cstudio'}));
  console.log(await call('studio_screenshot',{presentation:true,panel:'all',path:out+'.png'}));
}
export async function build(){
  if((await call('studio_status')).dirty)throw new Error('Save the current Studio work first.');
  await call('studio_open',{path:path.join(ROOT,'assets/skins/Catamp Silverplay.wsz')});
  await canvas({active:true,pressed:false,zoom:2,preview_playlist_height:145});
  await controlsGround();
  await paintReference();
  await details();
  await nativeCorrections();
  await refine(project);
  await fillMissedSpace();
  await repairContinuity(project);
  await finish();
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)await build();
