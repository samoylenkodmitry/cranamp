// Redraws the Fish Hat variation through Skin Studio's shared native pen.
import fs from 'node:fs';
import path from 'node:path';
import { ROOT, Pen, call } from '../../tools/skin-studio/moonlit.mjs';
const out=path.join(ROOT,'target/cat-skin-review');
const save=async (name,result)=>fs.writeFileSync(path.join(out,name+'.json'),JSON.stringify(result,null,2));
const status=await call('studio_status');
if(status.dirty) throw new Error('Save the current project before running this recipe');
await call('studio_open',{path:path.join(ROOT,'assets/skins/Catamp Silverplay.wsz')});
await call('studio_layers',{action:'select',id:'base'});
const fur={'#304355':'#4a2730','#687889':'#a85b3d','#9da8ad':'#f6a257','#d4d6ca':'#ffd18a','#f1ebd5':'#fff0c9','#b98190':'#ef8c9c','#1e222a':'#2d2032'};
for(const [sheet,w,h] of status.sheets.filter(([name])=>['main.bmp','eqmain.bmp','titlebar.bmp'].includes(name))){
 await call('studio_atlas',{sheet});
 for(const [from,to] of Object.entries(fur)){
  await call('studio_draw',{label:'Orange fur palette',mask_colors:[from],operations:[{op:'rect',x:0,y:0,width:w,height:h,fill:true,color:to}]});
 }
}
await call('studio_canvas',{surface:'skin',grid:false,guides:false,mask_colors:[],presentation:false,active:true,pressed:false});
await call('studio_layers',{action:'add',name:'02 Fish hats and snack demands'});
const ink='#102b40',mint='#a5e7cc',light='#e7ffdc',pink='#ef8c9c',orange='#f6a257';
const p=new Pen();
function fish(x,y,w,h){
 p.poly([[x+w-4,y+h/2],[x+w+5,y-1],[x+w+5,y+h+1]],ink);
 p.poly([[x+w-2,y+h/2],[x+w+3,y+2],[x+w+3,y+h-1]],pink);
 p.oval(x,y,w,h,ink).oval(x+1,y+1,w-2,h-2,mint);
 p.curve(x+5,y+2,x+w/2,y-2,x+w-5,y+2,light,1);
 p.oval(x+3,y+2,5,5,light).rect(x+4,y+3,2,3,ink);
 p.line(x+1,y+h-3,x+4,y+h-3,ink);
 p.curve(x+12,y+3,x+15,y+h/2,x+12,y+h-3,'#508ea3');
 p.poly([[x+15,y+2],[x+22,y-4],[x+23,y+2]],pink);
}
// A live fish has claimed the cat's head. The number eyes remain readable.
fish(39,8,43,12);
p.curve(48,21,43,27,38,29,pink,2).curve(75,20,80,25,84,25,pink,2);
p.rect(120,4,92,7,'#193e55').text(122,5,'FISH HAT FM',light,'small',1);
// Tiny sleeping cat gets a hat too, with one offended eye.
fish(174,119,18,8);
p.rect(157,127,2,1,ink).rect(164,127,2,1,ink);
// Snack demand sits in the open space above the EQ switches.
p.text(13,121,'NO THOUGHTS.',mint,'small',0).text(13,128,'ONLY SARDINES.',pink,'small',0);
// The standing cat wears a bib and holds an enormous fish sandwich.
p.poly([[57,188],[72,188],[70,199],[64,203],[59,198]],pink);
p.oval(59,191,11,7,light).text(61,192,'?',ink,'small',0);
fish(48,178,23,10);
p.oval(46,184,6,4,orange).oval(71,184,6,4,orange);
// A few bubbles tie the fish theme to the existing curved frame.
for(const [x,y,r] of [[15,42,3],[103,51,3],[228,120,3],[242,130,2]]){
 p.oval(x,y,r+2,r+2,'#508ea3').oval(x+1,y+1,r,r,'#193e55').rect(x+1,y+1,1,1,light);
}
await save('art',await call('studio_draw',{label:'Fish Hat FM',operations:p.ops,layers:[],states:'all',require_continuity:true}));
await call('studio_layers',{action:'add',name:'03 Playlist caption'});
await save('caption',await call('studio_draw',{label:'Snack queue',origin:'playlist.title',states:'all',operations:new Pen().rect(0,0,100,20,'#193e55').text(4,3,'THE SNACK QUEUE',mint,'small',0).ops}));
await call('studio_options',{playlist_colors:{Normal:'#c7e8e7',Current:'#ffd18a',NormalBG:'#193e55',SelectedBG:'#32627b',MbFG:'#c7e8e7',MbBG:'#193e55'}});
await save('validation',await call('studio_validate'));
await call('studio_canvas',{active:true,pressed:false,drawer:'none',grid:false,guides:false});
await save('gpu',await call('studio_screenshot',{panel:'all',presentation:true,path:path.join(out,'fish-hat-gpu.png')}));
await call('studio_project',{action:'save',path:path.join(out,'fish-hat-draft.cstudio')});
console.log('Fish Hat draft saved for visual review');
