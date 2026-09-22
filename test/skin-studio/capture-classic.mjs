// Read-only state sweep through the actual GPU; never changes source artwork.
import fs from 'node:fs/promises';
import path from 'node:path';
import {studio} from '../../tools/skin-studio/classic_migration.mjs';
const root=path.resolve(import.meta.dirname,'../..');
export async function captureClassic(name) {
 const dir=path.join(root,'target/classic-migration/final',name);
 await fs.mkdir(dir,{recursive:true});
 await studio('studio_project',{action:'open',path:path.join(root,'assets/skins',name+'.cstudio'),discard:true});
 const saved=(await studio('studio_status',{})).view;
 try {
  for(let n=0;n<4;n++) {
   const position=n*9;
   await studio('studio_state',{panel:'canvas',presentation:true,zoom:2,playback:1,active:!!(n&1),pressed:!!(n&2),volume:position,balance:position,position,scroll:position,eq:Array(11).fill(position)});
   await studio('studio_screenshot',{panel:'all',presentation:true,path:path.join(dir,'state-'+n+'.png')});
  }
  await studio('studio_state',{...saved,panel:'canvas',presentation:true,zoom:2,playback:0,active:true,pressed:false});
  await studio('studio_screenshot',{panel:'all',presentation:true,path:path.join(root,'assets/skins',name+'.png')});
  return name;
 } finally { await studio('studio_state',saved); }
}
