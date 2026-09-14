#!/usr/bin/env node
// Thin client for the MCP server inside the running Cranamp Skin Studio.
import fs from 'node:fs';
const name=process.argv[2]||'studio_status';
const argument=process.argv[3];
const args=argument?JSON.parse(argument.startsWith('@')?fs.readFileSync(argument.slice(1),'utf8'):argument):{};
const response=await fetch('http://127.0.0.1:18765/mcp',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/call',params:{name,arguments:args}})});
const body=await response.json();
for(const item of body.result?.content||[]){
  if(item.type==='text')console.log(item.text);
  if(item.type==='image'&&process.env.STUDIO_IMAGE_PATH){fs.writeFileSync(process.env.STUDIO_IMAGE_PATH,Buffer.from(item.data,'base64'));console.log(process.env.STUDIO_IMAGE_PATH);}
}
if(body.result?.isError||body.error)process.exitCode=1;
