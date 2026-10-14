import {Session,literalText} from './session.mjs';
const $=s=>document.querySelector(s), all=s=>[...document.querySelectorAll(s)];
let opened=await (await fetch('/open')).json(), session=new Session(opened.source), mode=1, projection=null, timer=null, focus=null;
const storageKey='weftext-editor-loop:'+location.origin;
function status(s){$('#status').textContent=s;}
function persist(){localStorage.setItem(storageKey,JSON.stringify({source:session.source,base:opened.source,file_id:opened.file_id}));}
function changed(){persist();clearTimeout(timer);if(!session.composing)timer=setTimeout(compile,200);status(`草稿 ${session.serial} · 未保存`);}
async function compile(){
  if(session.composing)return;
  const req={source:session.source,serial:session.serial};
  const response=await fetch('/compile',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(req)});const result=await response.json();
  if(!session.accept(result))return;
  projection=result;$('#diagnostic').textContent=result.diagnostic||result.error||'';
  status(result.ok?`草稿 ${session.serial} · 真实编译 · ${result.candidates.length} 个字面来源可回写`:`草稿 ${session.serial} · 编译失败，原文仍保留且可保存`);
  render();
}
function saveSelection(el){const s=getSelection();return s.rangeCount&&el.contains(s.anchorNode)?s.anchorOffset:0;}
function restoreSelection(el,offset){if(!el.firstChild)return;const r=document.createRange();r.setStart(el.firstChild,Math.min(offset,el.firstChild.textContent.length));r.collapse(true);getSelection().removeAllRanges();getSelection().addRange(r);}
function node(n){
  if(n.kind==='text'){
    const el=document.createElement('span');el.textContent=n.text;
    if(n.writable){
      el.dataset.origin=String(n.occurrence);el.dataset.from=String(n.from);el.tabIndex=0;
      if(mode===4){
        el.contentEditable='true';el.setAttribute('role','textbox');el.setAttribute('aria-label','正文 '+n.text.slice(0,20));
        let a=n.from,b=n.to,serial=session.serial;
        el.addEventListener('compositionstart',()=>session.beginComposition());
        el.addEventListener('compositionend',()=>{session.endComposition();changed();});
        el.addEventListener('input',()=>{try{const raw=literalText(el.textContent);session.replace(a,b,raw,serial);b=a+raw.length;serial=session.serial;changed();}catch(e){render();status(e.message);}});
        el.addEventListener('keydown',e=>{if(e.key==='Enter'){e.preventDefault();status('本原型的结构 Enter 尚未实现；请用源码视图。');}});
      }else if(mode===3){el.addEventListener('click',()=>showFocus(n));el.addEventListener('keydown',e=>{if(e.key==='Enter')showFocus(n);});}
    }
    return el;
  }
  const allowed=new Set(['p','div','span','section','article','h1','h2','h3','h4','h5','h6','strong','em','s','ul','ol','li','table','tbody','thead','tr','th','td','pre','code','blockquote','br','a']);
  if(n.kind==='element'){
    const el=document.createElement(allowed.has(n.tag)?n.tag:'div');
    if(n.tag==='a')el.setAttribute('title','链接仅展示；实验不自动导航');
    for(const child of n.children||[])el.append(node(child));return el;
  }
  if(n.kind==='frame'){const el=document.createElement('div');el.className='readonly';el.textContent='原生排版片段：本实验不提供回写';return el;}
  return document.createTextNode('');
}
function body(n){if(n.kind==='element'&&n.tag==='body')return n;for(const c of n.children||[]){const v=body(c);if(v)return v;}return null;}
function showFocus(n){focus={from:n.from,to:n.to,serial:session.serial};$('#focusSource').value=session.source.slice(n.from,n.to);$('#focusPanel').classList.remove('hidden');$('#focusSource').focus();}
function render(){
  all('[data-mode]').forEach(b=>b.setAttribute('aria-pressed',String(+b.dataset.mode===mode)));
  $('#source').classList.toggle('hidden',mode!==1);$('#visibleSource').classList.toggle('hidden',mode!==2);$('#flow').classList.toggle('hidden',mode<3);$('#focusPanel').classList.add('hidden');
  if(mode===1){if($('#source').value!==session.source)$('#source').value=session.source;return;}
  if(mode===2){
    const el=$('#visibleSource');if(document.activeElement===el)return;el.replaceChildren();
    const raw=new TextEncoder().encode(session.source);let ranges=[];
    if(projection?.ok&&session.accept(projection))for(const c of projection.candidates)for(const a of c.origin.ancestors){if(['Heading','Strong','Emph'].includes(a.kind))ranges.push(a);}
    // Author characters stay visible; styles come only from official source ancestors.
    const unique=[...new Map(ranges.map(r=>[`${r.start}:${r.end}`,r])).values()].sort((a,b)=>a.start-b.start||b.end-a.end);let position=0;
    for(const r of unique){if(r.start<position)continue;el.append(new TextDecoder().decode(raw.slice(position,r.start)));const span=document.createElement('span');span.className=r.kind.toLowerCase();span.textContent=new TextDecoder().decode(raw.slice(r.start,r.end));el.append(span);position=r.end;}
    el.append(new TextDecoder().decode(raw.slice(position)));return;
  }
  const active=document.activeElement?.dataset?.from,offset=active!==undefined?saveSelection(document.activeElement):0;
  $('#flow').replaceChildren();
  if(projection?.ok&&session.accept(projection)){
    const b=body(projection.tree);for(const c of b?.children||[])$('#flow').append(node(c));
    if(active!==undefined&&mode===4){const el=$(`#flow [data-from="${active}"]`);if(el){el.focus();restoreSelection(el,offset);}}
  }else{$('#flow').textContent='当前投影未就绪；草稿保留，可切换源码。';}
}
$('#source').addEventListener('compositionstart',()=>session.beginComposition());
$('#source').addEventListener('compositionend',()=>{session.endComposition();changed();});
$('#source').addEventListener('input',e=>{session.edit(e.target.value,[e.target.selectionStart,e.target.selectionEnd]);changed();});
$('#visibleSource').addEventListener('compositionstart',()=>session.beginComposition());
$('#visibleSource').addEventListener('compositionend',()=>{session.endComposition();changed();});
$('#visibleSource').addEventListener('input',e=>{session.edit(e.currentTarget.textContent);changed();});
all('[data-mode]').forEach(b=>b.addEventListener('click',()=>{if(session.composing){status('请先完成当前输入法组合。');return;}mode=+b.dataset.mode;focus=null;render();if(!projection||!session.accept(projection))compile();}));
$('#applyFocus').onclick=()=>{try{session.replace(focus.from,focus.to,$('#focusSource').value,focus.serial);$('#focusPanel').classList.add('hidden');changed();}catch(e){status(e.message);}};
$('#undo').onclick=()=>{if(session.undo()){render();changed();}};$('#redo').onclick=()=>{if(session.redo()){render();changed();}};
$('#restore').onclick=()=>{const saved=JSON.parse(localStorage.getItem(storageKey)||'null');if(saved){opened={source:saved.base,file_id:saved.file_id};session.edit(saved.source);render();changed();status('已恢复草稿和原 Base；外部新稿不会成为覆盖许可。');}};
$('#save').onclick=async()=>{
  if(session.composing)return;
  const serial=session.serial;
  const request=JSON.parse(localStorage.getItem(storageKey+':pending')||'null')||{source:session.source,base:opened.source,file_id:opened.file_id,operation:crypto.randomUUID()};
  localStorage.setItem(storageKey+':pending',JSON.stringify(request));
  try{
    const response=await fetch('/save',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(request)});const r=await response.json();
    if(r.state==='committed'){opened={source:request.source,file_id:r.file_id||request.file_id};localStorage.removeItem(storageKey+':pending');persist();status(serial===session.serial&&session.source===request.source?'已保存实验文件':'原保存已完成；当前草稿仍有未保存内容');}
    else {if(r.error)localStorage.removeItem(storageKey+':pending');status(r.error||r.state);}
  }catch{status('保存结果未知；再次点击将查询/重试原操作，不分配新操作。');}
};
document.addEventListener('keydown',e=>{if((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='z'){e.preventDefault();if(e.shiftKey)$('#redo').click();else $('#undo').click();}});
render();compile();
