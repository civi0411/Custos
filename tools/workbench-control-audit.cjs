/**
 * Static research inventory for sibling Orca and Open Science checkouts.
 * Usage: node tools/workbench-control-audit.cjs [AgentHub root] [output JSON]
 * Parses source only. It does not import or execute either reference project.
 * Counts are declarations, not rendered controls or a transitive call graph.
 */
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const ts = require('../ui/desktop/node_modules/typescript');
const base = path.resolve(process.argv[2] || path.join(__dirname, '../..'));
const outputPath = path.resolve(process.argv[3] || path.join(__dirname, '../docs/development/workbench-control-inventory.json'));
const projects = [['orca','orca/src/renderer/src'],['open-science','open-science/apps/desktop/src']];
const manifest=[], controls=[], actionObjects=[];
const enRoot = path.join(base, 'open-science/apps/desktop/src/i18n/locales/en');
const english = Object.fromEntries(fs.readdirSync(enRoot).filter(name => name.endsWith('.json'))
  .map(name => [path.basename(name, '.json'), JSON.parse(fs.readFileSync(path.join(enRoot, name), 'utf8'))]));
function resolveText(node, namespaces) {
  if (!node) return '';
  if (ts.isJsxExpression(node)) return resolveText(node.expression, namespaces);
  if (ts.isStringLiteralLike(node)) return node.text;
  if (!ts.isCallExpression(node)) return '';
  const name = node.expression.getText();
  if (name === 'translate' && node.arguments[1] && ts.isStringLiteralLike(node.arguments[1])) return node.arguments[1].text;
  if (name !== 't' || !node.arguments[0] || !ts.isStringLiteralLike(node.arguments[0])) return '';
  const key = node.arguments[0].text;
  const [explicit, rest] = key.includes(':') ? key.split(':', 2) : [null, key];
  for (const ns of explicit ? [explicit] : namespaces) {
    const value = rest.split('.').reduce((current, part) => current?.[part], english[ns]);
    if (typeof value === 'string') return value;
  }
  return '';
}

const squash = (s,n=240) => s.replace(/\s+/g,' ').trim().slice(0,n);
function walk(dir) { return fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=> e.isDirectory() && !['node_modules','mobile','__tests__','__fixtures__'].includes(e.name) ? walk(path.join(dir,e.name)) : e.isFile() && /\.(tsx|jsx|ts)$/.test(e.name) && !/\.(test|spec|stories|fixture)\./.test(e.name) ? [path.join(dir,e.name)] : []); }
for (const [project,root] of projects) for (const file of walk(path.join(base,root)).sort()) {
 const text=fs.readFileSync(file,'utf8'), rel=path.relative(base,file), sf=ts.createSourceFile(file,text,ts.ScriptTarget.Latest,true,/\.(tsx|jsx)$/.test(file)?ts.ScriptKind.TSX:ts.ScriptKind.TS);
 const namespaces = [];
 function translations(node) {
  if (ts.isCallExpression(node) && node.expression.getText(sf) === 'useTranslation') {
   const arg = node.arguments[0];
   if (arg && ts.isStringLiteralLike(arg)) namespaces.push(arg.text);
   else if (arg && ts.isArrayLiteralExpression(arg)) for (const item of arg.elements) if (ts.isStringLiteralLike(item)) namespaces.push(item.text);
  }
  ts.forEachChild(node, translations);
 }
 if (project === 'open-science') translations(sf);
 manifest.push({project,path:rel,sha256:crypto.createHash('sha256').update(text).digest('hex'),parseDiagnostics:sf.parseDiagnostics.length});
 function visit(node) {
  if(ts.isObjectLiteralExpression(node)) {
   const fields={};
   for(const p of node.properties) if(ts.isPropertyAssignment(p)) fields[p.name.getText(sf).replace(/['"]/g,'')]=squash(p.initializer.getText(sf),240);
   const actionKeys=Object.keys(fields).filter(k=>/^(onSelect|onClick|action|execute|handler|run)$/.test(k));
   if(actionKeys.length && ['label','title','id','name'].some(k=>k in fields)) actionObjects.push({project,path:rel,line:sf.getLineAndCharacterOfPosition(node.getStart(sf)).line+1,fields:Object.fromEntries(Object.entries(fields).filter(([k])=>['id','label','title','name','disabled','when','visible',...actionKeys].includes(k)))});
  }
  if(ts.isJsxOpeningElement(node)||ts.isJsxSelfClosingElement(node)) {
   const tag=node.tagName.getText(sf), props={}; let spreads=false;
   for(const a of node.attributes.properties) if(ts.isJsxAttribute(a)) props[a.name.getText(sf)]=a.initializer?squash(a.initializer.getText(sf),600):true; else spreads=true;
   const events=Object.keys(props).filter(k=>/^on(Click|Change|Submit|Select|ValueChange|CheckedChange|PointerDown|KeyDown|DoubleClick|ContextMenu|Drop|DragStart)$/.test(k));
   const native=/^(button|input|select|textarea|a|summary|form)$/.test(tag);
   const custom=/(Button|MenuItem|MenuCheckboxItem|MenuRadioItem|Toggle|Switch|Slider|SelectItem|RadioGroupItem|Checkbox|DropdownItem|ActionItem|TabTrigger|TabsTrigger|CommandItem)$/.test(tag);
   if(native||custom||events.length||props.role==='"button"'||props.role==='"tab"') {
    const line=sf.getLineAndCharacterOfPosition(node.getStart(sf)).line+1;
    let displayLabel = '';
    for (const attr of node.attributes.properties) if (ts.isJsxAttribute(attr) && ['aria-label', 'title'].includes(attr.name.getText(sf))) {
      displayLabel = resolveText(attr.initializer, namespaces) || displayLabel;
    }
    if (!displayLabel && ts.isJsxOpeningElement(node) && ts.isJsxElement(node.parent)) {
      const bits = [];
      for (const child of node.parent.children) {
        if (ts.isJsxText(child)) bits.push(child.text);
        else if (ts.isJsxExpression(child)) bits.push(resolveText(child, namespaces));
      }
      displayLabel = bits.join(' ');
    }
    let direct=[]; if(ts.isJsxOpeningElement(node)&&ts.isJsxElement(node.parent)) for(const c of node.parent.children) { if(ts.isJsxText(c)) direct.push(c.text); else if(ts.isJsxExpression(c)&&c.expression) direct.push('{'+squash(c.expression.getText(sf),100)+'}'); }
    let component=''; for(let p=node.parent;p;p=p.parent) { if(ts.isFunctionDeclaration(p)&&p.name){component=p.name.text;break;} if(ts.isVariableDeclaration(p)&&p.initializer&&(ts.isArrowFunction(p.initializer)||ts.isCallExpression(p.initializer))){component=p.name.getText(sf);break;} }
    const guards=[]; for(let p=node.parent;p && guards.length<3;p=p.parent) if(ts.isConditionalExpression(p)||ts.isBinaryExpression(p)&&p.operatorToken.kind===ts.SyntaxKind.AmpersandAmpersandToken) guards.push(squash(ts.isConditionalExpression(p)?p.condition.getText(sf):p.left.getText(sf),180));
    controls.push({project,path:rel,line,component,tag,displayLabel:squash(displayLabel,180)||null,label:squash(direct.join(' '),180),title:props.title||null,ariaLabel:props['aria-label']||null,disabled:props.disabled||null,events:Object.fromEntries(Object.keys(props).filter(key => /^on[A-Z]/.test(key)).map(e=>[e,squash(String(props[e]),240)])),guards,spreads,controlProps:Object.fromEntries(['type','role','href','value','checked','aria-expanded','aria-pressed','aria-selected','readOnly'].filter(k=>k in props).map(k=>[k,squash(String(props[k]),180)]))});
   }
  }
  ts.forEachChild(node,visit);
 }
 visit(sf);
}
const summary=projects.map(([project])=>({project,sourceFiles:manifest.filter(x=>x.project===project).length,jsxFiles:manifest.filter(x=>x.project===project&&/\.(tsx|jsx)$/.test(x.path)).length,actionObjects:actionObjects.filter(x=>x.project===project).length,controlDeclarations:controls.filter(x=>x.project===project).length,filesWithControls:new Set(controls.filter(x=>x.project===project).map(x=>x.path)).size,parseDiagnostics:manifest.filter(x=>x.project===project).reduce((s,x)=>s+x.parseDiagnostics,0)}));
fs.writeFileSync(outputPath,JSON.stringify({method:'TypeScript JSX AST declaration inventory; not rendered unique controls or a transitive call graph',excluded:'test/spec/story/fixture files and mobile directories; CSS/data-driven registry-only actions may not appear',summary,thirdPartyNotices:projects.map(([project])=>({project,license:fs.readFileSync(path.join(base,project,'LICENSE'),'utf8')})),files:manifest,actionObjects,controls},null,2)+'\n');
console.log(JSON.stringify(summary));
for(const project of ['orca','open-science']) { const counts={};for(const c of controls.filter(x=>x.project===project)){const p=c.path.split('/');const key=project==='orca'?p.slice(4,6).join('/'):p.slice(5,7).join('/');counts[key]=(counts[key]||0)+1;}console.log(project,JSON.stringify(Object.entries(counts).sort((a,b)=>b[1]-a[1]).slice(0,20))); }

// A compact navigation aid; the JSON remains the exact declaration evidence.
const panelPath = outputPath.replace(/\.json$/, '-panels.md');
if (panelPath !== outputPath) {
  const escape = value => String(value || '').replace(/\|/g, '\\|').replace(/[\r\n]/g, ' ').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  const lines = ['# Desktop panel and control source index', '',
    'Generated from static source declarations, not from runtime screenshots. Counts include delegated controls and interaction surfaces; they are not distinct rendered actions.', '',
    `Exact declarations, handlers, guards and hashes: [inventory](${path.basename(outputPath)}).`, ''];
  for (const [project] of projects) {
    lines.push(`## ${project}`, '', '| Source panel/component | Declarations | Example static labels |', '|---|---:|---|');
    const grouped = new Map();
    for (const control of controls.filter(item => item.project === project)) {
      if (!grouped.has(control.path)) grouped.set(control.path, []);
      grouped.get(control.path).push(control);
    }
    for (const [source, rows] of grouped) {
      const target = path.relative(path.dirname(panelPath), path.join(base, source)).split(path.sep).join('/');
      const labels = [...new Set(rows.map(row => row.displayLabel).filter(Boolean))].slice(0, 3).join('; ');
      lines.push(`| [${escape(source.replace(`${project}/`, ''))}](${target}) | ${rows.length} | ${escape(labels || 'Dynamic label or delegated control; inspect inventory')} |`);
    }
    lines.push('');
  }
  fs.writeFileSync(panelPath, `${lines.join('\n')}\n`);
}
