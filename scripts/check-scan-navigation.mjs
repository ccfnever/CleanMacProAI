// Run against the Vite dev server with Playwright installed (or PLAYWRIGHT_MODULE set).
// Native commands are deferred mocks: no filesystem scan or cleanup is performed.
import assert from 'node:assert/strict';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE ? `${process.env.PLAYWRIGHT_MODULE}/index.mjs` : 'playwright');
(async()=>{
 const browser=await chromium.launch({channel:'chrome',headless:true});
 try {
 const page=await browser.newPage({viewport:{width:1280,height:850}});const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.addInitScript(()=>{
 window.isTauri=true;window.calls={};window.pending={};
 window.__TAURI_INTERNALS__={invoke:async(command)=>{
 window.calls[command]=(window.calls[command]||0)+1;
 if(command==='get_disk_info')return {volume_name:'Test',total_bytes:1000,available_bytes:500,usage_percent:50};
 if(command==='get_space_map_progress') throw Error('poll unavailable');
 return await new Promise(resolve=>window.pending[command]=resolve);
 }};
 });
 await page.goto(process.env.APP_URL || 'http://127.0.0.1:1420');
 const fixtures=await page.evaluate(async()=>{const d=await import('/src/lib/demoData.ts');return {scan:d.demoScanResult,map:d.demoSpaceMapResult}});
 for(const [label,command,result] of [['安心清理','scan_system',fixtures.scan],['空间地图','analyze_space_map',fixtures.map],['应用管理','list_installed_apps',[{name:'Test App',app_path:'/Applications/Test.app',bundle_id:'test.app',app_size:0,related_size:0,related_files:[],is_system_app:false}]],['启动项管理','list_startup_items',{items:[],warnings:[]}]]){
 const nav=page.getByRole('button',{name:new RegExp('^'+label)});
 await nav.click();await page.waitForFunction(c=>!!window.pending[c],command);
 assert.equal(await nav.locator('.nav-scanning').count(),1);
 await page.getByRole('button',{name:'外观与设置',exact:true}).click();await page.locator('.settings-page').waitFor();
 assert.equal(await nav.locator('.nav-scanning').count(),1);
 await nav.click();await page.waitForTimeout(400);
 assert.equal(await page.evaluate(c=>window.calls[c],command),1);
 await page.getByRole('button',{name:'外观与设置',exact:true}).click();
 await page.evaluate(([c,r])=>window.pending[c](r),[command,result]);
 if(label==='应用管理'){await page.waitForFunction(()=>!!window.pending.inspect_installed_app);assert.equal(await nav.locator('.nav-scanning').count(),1);await page.evaluate(app=>window.pending.inspect_installed_app({...app,app_size:1024}),result[0]);}
 await page.waitForFunction(label=>!Array.from(document.querySelectorAll('.nav-item')).find(e=>e.textContent.includes(label)).querySelector('.nav-scanning'),label);
 await nav.click();await page.waitForTimeout(500);
 assert.equal(await page.evaluate(c=>window.calls[c],command),1);
 if(label==='空间地图')assert.ok(await page.locator('canvas').count()>0);
 console.log('PASS: '+label+' auto-start, background scan, retained result, no duplicate, sidebar status');
 }
 assert.equal(await page.evaluate(()=>window.calls.cancel_space_map||0),0);
 assert.deepEqual(errors,[]);

 }finally{await browser.close()}
})().catch(e=>{console.error(e);process.exit(1)});
