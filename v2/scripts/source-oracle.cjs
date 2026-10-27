// Validation only. Executes the unchanged TSX with recording React/Raycast boundaries.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const v=path.resolve(__dirname,'..');
const ts=require(path.join(v,'.scratch/v1/node_modules/typescript'));
const marked=require(path.join(v,'.scratch/v1/node_modules/marked'));
const source=fs.readFileSync(path.join(v,'fixtures/own/src/preview-markdown.tsx'),'utf8');
const code=ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX,target:ts.ScriptTarget.ES2022}}).outputText;
const host=name=>({name});
const Action=Object.assign(host('Action'),{SubmitForm:host('Action.SubmitForm'),CopyToClipboard:host('Action.CopyToClipboard')});
const Detail=Object.assign(host('Detail'),{Metadata:Object.assign(host('Detail.Metadata'),{Label:host('Detail.Metadata.Label')})});
const Form=Object.assign(host('Form'),{TextArea:host('Form.TextArea')});
let values=[],cursor=0,stack=[],calls=[];
const jsx=(type,props)=>({type,props:props||{}});
const api={Action,ActionPanel:host('ActionPanel'),Detail,Form,useNavigation:()=>({push:el=>{calls.push({route:'push'});stack.push(el)},pop:()=>{calls.push({route:'pop'});stack.pop()}})};
const react={useState(initial){const i=cursor++;if(!(i in values))values[i]=initial;return [values[i],next=>{values[i]=typeof next==='function'?next(values[i]):next}];}};
const mod={exports:{}};
vm.runInNewContext(code,{exports:mod.exports,module:mod,require(id){if(id==='@raycast/api')return api;if(id==='react')return react;if(id==='react/jsx-runtime')return {jsx,jsxs:jsx};if(id==='marked')return marked;throw Error('unrecorded import '+id)}},{filename:'actual-preview-markdown.cjs'});
function render(){cursor=0;let el=stack.at(-1)||jsx(mod.exports.default,{});while(typeof el.type==='function')el=el.type(el.props);return el;}
function walk(el,name){if(!el)return [];let out=el.type?.name===name?[el]:[];for(const child of [el.props?.actions,el.props?.metadata,...[el.props?.children].flat()].flat()){if(child&&typeof child==='object')out.push(...walk(child,name));}return out;}
function reset(){values=[];stack=[];calls=[];return render()}
const initial=reset();
const inputs=[
 '', ' ', '\t\n\r', '\ufeff', '\u0085', '# Heading 1', '## Two\n\n### Three', 'Heading\n=======', 'Subtitle\n----',
 'plain text', 'first\nsecond', 'first  \nsecond', 'first\\\nsecond', 'first\r\nsecond\rthird', 'a\n\nb',
 '**bold** and *italic*', '__bold__ _italic_', '***both***', '~~deleted~~', '~single~', 'a_b_c', 'a*b*c', '\\*literal\\*',
 '`inline <code> & "`', '`` a ` b ``', '` x `', '```js\nconst x = "<&>";\n```', '~~~python\nprint(1)\n~~~',
 '```rust extra\nfn main() {}\n```', '```\nunterminated', '    indented\n    code', '---', '***', '___',
 '> quote\n> line', '> one\n>\n> two', '> > nested', '- a\n- b', '* a\n* b', '+ a\n+ b', '1. a\n2. b', '3. a\n4. b',
 '- a\n\n- b', '- a\n  - nested\n  - more\n- b', '- [ ] task\n- [x] done\n- [X] also',
 '| a | b |\n| - | - |\n| c | d |', '| left | mid | right |\n| :--- | :---: | ---: |\n| x | y | z |',
 '[Link](https://example.com)', '[a](https://example.com "title")', '[a](<https://example.com/a%20b>)',
 '<https://example.com>', '<a@example.com>', 'https://example.com', 'www.example.com', 'mail a@example.com!',
 '![alt](https://example.invalid/x.png)', '![a *b*](img.png "title")', '[ref][x]\n\n[x]: /url "title"',
 '<div>raw & text</div>', '<b>inline</b>', '<script>alert(1)</script>', 'a & b < c > d "quote" \'apostrophe\'',
 '&amp; &copy; &#65; &#x1f600;', '😀 Қазаq 日本\nnext', '\u00a0one\u2003two\ufeffthree', '\u0085one\u0085two',
 'trailing\n', 'a\n\n', '\n\na', 'a\u0000b', '- **bold**\n- `code`', '> - a\n> - b',
 'line\n# heading\ntext', 'a\n---\nb', '    <a>&\n\n    b', '```a&b\n<>&\'"\n```',
 '[](/empty)', '![](/empty)', '[safe](javascript:alert(1))', '<!-- comment -->\n\ntext',
 '![only](https://example.invalid/only.png)', '~~a\nb~~', 'a\t\tb', 'a\n\n\n b',
 '<pre>\n*raw*\n</pre>\n\ntext', '- a\n\n  paragraph\n\n- b', '1. a\n\n2. b',
 '[a](a?x=1&y=2)', '[a](a%20b)', '[a](a\\(b\\))', '![a & b](a?x=1&y=2)',
];
inputs.push(
 '**bold**  ', '[link](/x)  ', '`code`  ', '<b>x</b>  ', '![a \*b\*](x)  ',
 '- [x] **done**\n\n- [ ] second', '- [x] one\n\n  two',
 '| a | b |\n| --- | --- |', '| a | b |\n| --- | --- |\n| one |\n| x | y | z |',
 '> quote\n\ntext', '> a\n> > b\n> c', 'a\nb\n\nc', '***a _b_***',
 '<https://example.com/a?b=1&c=2>', 'https://example.com/a(b).', 'www.example.com/test,',
 'A < B and &broken; &notvalid;', '[a](a \"b & c\")', '[a](<a b>)', '[a](a\\b)',
 '![a [b] c](x)', '![a `b`](x)', '![a &amp; b](x)',
 'foo_bar_baz', '~~strike~~ and ~single~', '~~foo~~bar', 'a___b___c',
 '```\n\n```', '    indented\n\n    second\n', '```\n<&amp;>\n```',
 '<div>\nhello\n</div>\n\nnext', '<!-- comment -->', 'text\n\n    code\n',
 '[x]: /target\n\n[x]', '[nope][missing]', '<user+tag@example.com>',
 'https://example.com/?a=1&b=2', 'http://example.com/a_(b)',
 'para\n  continuation  ', 'line \n next', '- one\n  continuation'
 );
inputs.push(
 '![a \\*b\\*](x)', '![a \\[b\\]](x)', '![a ![b](y)](x)',
 '<a href="/">https://example.com</a>', '<code>a & b www.example.com</code>',
 '<kbd>*x* & y</kbd>', '~~a~~  ', '*x*\n  y', 'foo\\_bar',
 '- one\n\n  > quote', '1. one\n   continuation', '![a "b"](x "t")',
 'https://example.com/path?x=1&y=2.', 'www.example.com/a?x=1&y=2',
 '[a](http://example.com/😀)', '[a](http://example.com/a%20b)',
 '[![a](x)](y)', '~~foo~bar~~', '~~foo~~bar~~baz~~',
 'text <span>www.example.com</span>', '<textarea>www.example.com & x</textarea>',
 '| a \\| b | c |\n| - | - |\n| x | y |',
 '[a](x "one\\\"two")', '\\&amp;', '[r]: /x\n\n![a][r]'
 );
inputs.push('[a](a[b])', 'https://example.com/😀', 'www.example.com/日本', '\\\\&amp;');
for(let n=1;n<=6;n++)inputs.push('#'.repeat(n)+' h'+n);
for(const ws of ['\t','\v','\f','\r','\n',' ','\u00a0','\u1680','\u2000','\u200a','\u2028','\u2029','\u202f','\u205f','\u3000','\ufeff','\u0085'])inputs.push(ws+'a'+ws+'b'+ws);
const cases=inputs.map((input,index)=>{reset();let tree=render();walk(tree,'Form.TextArea')[0].props.onChange(input);tree=render();walk(tree,'Action.SubmitForm')[0].props.onSubmit({markdown:'forged ignored by actual closure'});tree=render();const accepted=tree.type.name==='Detail';return {index,input,accepted,html:accepted?walk(tree,'Action.CopyToClipboard')[1].props.content:null,markdown:accepted?tree.props.markdown:null,statistics:accepted?walk(tree,'Detail.Metadata.Label').map(x=>({title:x.props.title,text:x.props.text})):null,actions:walk(tree,'Action').map(x=>x.props.title).concat(walk(tree,'Action.CopyToClipboard').map(x=>x.props.title))};});
// Execute source navigation, Clear, and committed copy bindings explicitly.
reset();walk(render(),'Form.TextArea')[0].props.onChange('  # draft\n😀  ');walk(render(),'Action.SubmitForm')[0].props.onSubmit({});
let preview=render();const copied=walk(preview,'Action.CopyToClipboard').map(x=>({title:x.props.title,content:x.props.content}));
walk(preview,'Action')[0].props.onAction();const restored=walk(render(),'Form.TextArea')[0].props.value;
walk(render(),'Action')[0].props.onAction();const cleared=walk(render(),'Form.TextArea')[0].props.value;
const output={sourceSha:'3cd8b99d5919ca6098db3edc08ea1a9f59ec0600',markedVersion:require(path.join(v,'.scratch/v1/node_modules/marked/package.json')).version,initial:{title:initial.props.navigationTitle,field:{...walk(initial,'Form.TextArea')[0].props,onChange:undefined}},navigation:{copied,restored,cleared,calls},cases};
fs.writeFileSync(path.join(v,'fixtures/source-oracle.json'),JSON.stringify(output,null,2)+'\n');
console.log(`Executed actual own TSX + locked marked: ${cases.length} inputs, Clear/Back/copy bindings, empty defaults`);
