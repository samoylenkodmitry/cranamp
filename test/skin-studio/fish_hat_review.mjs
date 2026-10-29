import fs from 'node:fs';
import path from 'node:path';
import {ROOT,call} from '../../tools/skin-studio/moonlit.mjs';
const out=path.join(ROOT,'target/cat-skin-review');
const original=(await call('studio_status')).view;
for (const [name,view] of [
 ['active-up',{active:true,pressed:false}],
 ['active-down',{active:true,pressed:true}],
 ['inactive-up',{active:false,pressed:false}],
 ['inactive-down',{active:false,pressed:true}],
 ['low',{active:true,pressed:false,eq:Array(11).fill(0),volume:0,balance:0,position:0}],
 ['high',{active:true,pressed:true,eq:Array(11).fill(27),volume:27,balance:27,position:27}],
 ['mixed',{active:true,pressed:false,eq:[0,27,3,24,6,21,9,18,12,15,14],preview_playlist_height:174}],
 ['tall',{active:true,pressed:false,eq:[27,0,27,0,27,0,27,0,27,0,27],preview_playlist_height:203}]
]) {
 await call('studio_canvas',view);
 const r=await call('studio_screenshot',{panel:'all',presentation:true,path:path.join(out,name+'.png')});
 fs.writeFileSync(path.join(out,name+'.json'),JSON.stringify(r,null,2));
}
await call('studio_canvas',{active:original.active,pressed:original.pressed,eq:original.eq,volume:original.volume,balance:original.balance,position:original.position,preview_playlist_height:original.preview_playlist_height});
console.log('Captured four control states, extremes, mixed EQ and playlist resize views');
