// Validation only: actual pinned Surface/Form/Detail/Markdown with jsdom and recorded host effects.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const {createRequire}=require('node:module');const v=path.resolve(__dirname,'..');const req=createRequire(path.join(v,'validation/package.json'));
const {JSDOM}=req('jsdom');
// jsdom resources and script execution are disabled. No image/network/browser/clipboard effects.
const dom=new JSDOM('<!doctype html><html><body></body></html>',{url:'https://local.invalid/'});
for(const k of ['window','document','navigator','Element','HTMLElement','HTMLTextAreaElement','HTMLInputElement','DOMParser','Event','KeyboardEvent','MouseEvent'])Object.defineProperty(globalThis,k,{value:dom.window[k],configurable:true});
globalThis.IS_REACT_ACT_ENVIRONMENT=true;
const React=req('react'),{createRoot}=req('react-dom/client'),{Simulate}=req('react-dom/test-utils');const {act}=React;
const ts=req('typescript'),effects=[];let copyReply={ok:true};let copyReject=false;
window.tezbar={clipboardWriteText:async text=>{effects.push(['clipboard',text]);if(copyReject)throw Error('PRIVATE host error');return copyReply},setSuppressBlurHide:async value=>{effects.push(['suppress-blur',value]);return {ok:true}}};
const nav=React.createContext({push(){},pop(){}});
const hooks={NavigationContext:nav,setRuntimeContext:(...args)=>effects.push(['context',...args]),useNavigation:()=>React.useContext(nav)};
for(const name of ['Clipboard','environment','getPreferenceValues','open','showHUD','showToast','useCachedPromise','useFetch','usePromise'])hooks[name]=()=>{throw Error('Unexpected hook/host effect '+name)};
const cache=new Map();let patchedMarkdown=false;
function load(relative){
 if(cache.has(relative))return cache.get(relative).exports;
 const filename=path.join(v,'fixtures/beta',relative),mod={exports:{}};cache.set(relative,mod);
 const code=ts.transpileModule(fs.readFileSync(patchedMarkdown&&relative==='src/renderer/ui/Markdown.tsx'?path.join(v,'integration/Markdown.tsx'):filename,'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX,target:ts.ScriptTarget.ES2022,esModuleInterop:true}}).outputText;
 function local(id){
  if(id==='./hooks')return hooks;
  if(id.endsWith('/ui/primitives'))return new Proxy({cx:(...a)=>a.filter(Boolean).join(' ')},{get:(obj,key)=>obj[key]||((props)=>React.createElement('span',null,props.children))});
  if(id.endsWith('/colorConverter'))return {buildColorFormatRows:()=>{throw Error('Out-of-scope color presentation called')}};
  if(['./grid-runtime','./list-runtime'].includes(id))return new Proxy({},{get:()=>()=>{throw Error('Out-of-scope grid/list called')}});
  if(id.startsWith('.')){const target=path.posix.normalize(path.posix.join(path.posix.dirname(relative),id));for(const ext of ['.tsx','.ts'])if(fs.existsSync(path.join(v,'fixtures/beta',target+ext)))return load(target+ext);throw Error('Unrecorded relative import '+id);}
  return req(id);
 }
 vm.runInThisContext('(function(require,module,exports){'+code+'\n})',{filename})(local,mod,mod.exports);return mod.exports;
}
const base='src/renderer/src/raycast-api/';
const form=load(base+'form-runtime.tsx'),detail=load(base+'detail-runtime.tsx'),surface=load(base+'index.tsx'),{Markdown}=load('src/renderer/ui/Markdown.tsx');
const wire=JSON.parse(fs.readFileSync(path.join(v,'fixtures/wire.json'),'utf8'));
let checks=0;const check=(condition,message)=>{assert.ok(condition,message);checks++};
let root,container;
async function mount(component,props){container=document.createElement('div');document.body.appendChild(container);root=createRoot(container);await act(async()=>root.render(React.createElement(component,props)));}
async function update(component,props){await act(async()=>root.render(React.createElement(component,props)));}
async function unmount(){await act(async()=>root.unmount());container.remove();}
async function main(){
 // Actual reconciliation: heartbeat and old callback responses must not replace a newer local draft.
 const field={key:'markdown:Form.TextArea',type:'Form.TextArea',id:'markdown',title:'Markdown Content',value:'',actionId:'field-cap'};
 check(form.reconcileFormValues([{...field,value:undefined}],{markdown:'new queued'},[field]).markdown==='new queued','callback omission preserves draft');
 check(form.reconcileFormValues([{...field,value:''}],{markdown:'new queued'},[field]).markdown==='','controlled clear resets draft');
 check(form.reconcileFormValues([{...field,actionId:undefined}],{markdown:'submit draft'},[{...field,actionId:undefined}]).markdown==='submit draft','submit-only heartbeat preserves draft');
 const invocations=[],searches=[];let onBack=0;
 let props={sessionId:'native-extension:test',extensionId:'com.tezbar.native.markdown-preview',commandName:'preview-markdown',...wire.editor,onBack:()=>onBack++,onSearchTextChanged:x=>searches.push(x),onLoadMore:()=>{throw Error('Unexpected load more')},onInvokeAction:(...args)=>invocations.push(args)};
 await mount(surface.ExtensionRuntimeSurface,props);
 const area=container.querySelector('textarea');check(!!area,'actual Form renders textarea');check(area.value==='','actual Form starts empty');check(area.placeholder===wire.editor.root.children[0].props.placeholder,'source placeholder preserved');
 await act(async()=>Simulate.change(area,{target:{value:'# newer\n😀'}}));
 check(invocations.at(-1)[0]===wire.editor.root.children[0].props.actionId,'callback capability');check(invocations.at(-1)[1].value==='# newer\n😀','callback formValues.value');
 const oldResponse=structuredClone(wire.editor);delete oldResponse.root.children[0].props.value;
 await update(surface.ExtensionRuntimeSurface,{...props,...oldResponse});check(container.querySelector('textarea').value==='# newer\n😀','actual reconciliation protects newer draft');
 await act(async()=>Simulate.submit(container.querySelector('form')));
 check(invocations.at(-1)[0]===wire.editor.actions[0].id,'submit capability');check(invocations.at(-1)[1].markdown==='# newer\n😀','submit complete map');
 await update(surface.ExtensionRuntimeSurface,{...props,...wire.editor});check(container.querySelector('textarea').value==='','actual clear controlled response');
 await update(surface.ExtensionRuntimeSurface,{...props,...wire.preview});
 check(container.textContent.includes('Markdown Preview'),'Detail title');check(container.querySelector('h1')?.textContent==='Heading 1','actual beta Markdown receives raw Detail text');
 check(container.textContent.includes('Characters'),'actual metadata parser');check(container.textContent.includes('11'),'actual metadata values');
 const back=[...container.querySelectorAll('button')].find(b=>b.textContent==='Back');await act(async()=>back.click());check(invocations.at(-1)[0]===wire.preview.root.props.backActionId,'Surface Back routes committed capability');check(onBack===0,'preview Back does not dispose session');
 await act(async()=>window.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true})));check(invocations.at(-1)[0]===wire.preview.actions[0].id,'Detail primary Enter');
 await act(async()=>window.dispatchEvent(new KeyboardEvent('keydown',{key:'k',metaKey:true,bubbles:true})));check(container.textContent.includes('Copy HTML'),'actual action overlay');
 check(searches.length===0,'Form/Detail do not send list search');await unmount();
 // Actual Markdown parse, highlight, sanitization and DOM effects. All OS effects recorded.
 const text='# Title\n\nline one\nline two\n\n| a | b |\n| - | - |\n| x | y |\n\n- [x] task\n\n~~gone~~ **bold** *italic*\n\n```js\nconst x = 1;\n```\n\n<a href="https://example.invalid">link</a><script>evil()</script><img src="https://example.invalid/x" onerror="evil()">';
 await mount(Markdown,{text});
 check(!!container.querySelector('h1'),'heading');check(!!container.querySelector('br'),'breaks true');check(!!container.querySelector('table'),'GFM table');check(!!container.querySelector('input[type=checkbox]'),'task lists');check(!!container.querySelector('del'),'strikethrough');check(!!container.querySelector('strong')&&!!container.querySelector('em'),'inline styles');check(!!container.querySelector('.hljs-keyword'),'actual highlight.js');check(!container.querySelector('script'),'actual DOMPurify strips scripts');check(!container.querySelector('img')?.hasAttribute('onerror'),'actual sanitizer strips handler');
 const anchor=container.querySelector('a');check(anchor.target==='_blank'&&anchor.rel==='noopener noreferrer','actual link presentation');
 const copy=container.querySelector('.tezbar-md-code-copy');check(!!copy,'actual code-block copy presentation');await act(async()=>copy.click());check(effects.some(e=>e[0]==='clipboard'&&e[1]==='const x = 1;\n'),'code copy recorded, never system clipboard');
 check(container.querySelector('img')?.src==='https://example.invalid/x','image source retained; network loading disabled');await unmount();
 await mount(Markdown,{text:'~~~js\nconst open = 1;',streaming:true});check(!!container.querySelector('pre code'),'streaming unclosed fence');check(!container.querySelector('.tezbar-md-code-copy'),'streaming omits copy decoration');await unmount();
 // Expose pinned host's weak null handling; it is NOT claimed to meet strict NativeV2 ack semantics.
 copyReply=null;await mount(Markdown,{text:'```\nnull ack\n```'});await act(async()=>container.querySelector('.tezbar-md-code-copy').click());check(container.querySelector('.tezbar-md-code-copy').textContent==='Copied','known pinned host gap: null displays Copied');await unmount();
 const pinnedChecks=checks;
 // The required reviewable host patch fixes that control; the beta fixture remains exact.
 patchedMarkdown=true;cache.delete('src/renderer/ui/Markdown.tsx');const FixedMarkdown=load('src/renderer/ui/Markdown.tsx').Markdown;
 for(const reply of [null,true,false,{}, {ok:true,error:'PRIVATE'}, {ok:true,error:false}, {ok:true,success:false}, {ok:true,success:null}, {ok:false}]){
  copyReply=reply;copyReject=false;await mount(FixedMarkdown,{text:'```\ncheck ack\n```'});await act(async()=>container.querySelector('.tezbar-md-code-copy').click());check(container.querySelector('.tezbar-md-code-copy').textContent==='Failed','patched host rejects malformed/contradictory ack');await unmount();
 }
 for(const reply of [{ok:true},{ok:true,error:null},{ok:true,success:true}]){
  copyReply=reply;await mount(FixedMarkdown,{text:'```\ncheck ack\n```'});await act(async()=>container.querySelector('.tezbar-md-code-copy').click());check(container.querySelector('.tezbar-md-code-copy').textContent==='Copied','patched host accepts positive ack');await unmount();
 }
 copyReject=true;await mount(FixedMarkdown,{text:'```\ntransport error\n```'});await act(async()=>container.querySelector('.tezbar-md-code-copy').click());check(container.querySelector('.tezbar-md-code-copy').textContent==='Failed','patched host transport error is private failure');await unmount();
 console.log(`Pinned beta renderer: ${pinnedChecks} assertions; required host ack patch: ${checks-pinnedChecks} assertions; actual Surface/Form/Detail/Markdown and locked libraries; jsdom and OS boundaries explicit`);
 dom.window.close();
}
main().catch(e=>{console.error(e);dom.window.close();process.exitCode=1});
