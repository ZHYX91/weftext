// A bounded literal edit, not the production AST inverse editor.
export function literalText(source) {
  if (!source || /[\x00-\x1f\x7f]/.test(source)) throw Error('此实验仅回写非空单行文字；请用源码编辑结构。');
  return source.replace(/[!-/:-@\[-`{-~]/g, '\\$&');
}

// One source buffer and history shared by every prototype view.
export class Session {
  constructor(source) { this.source=source; this.serial=0; this.undoStack=[]; this.redoStack=[]; this.composing=false; this.selection=[0,0]; }
  snapshot() { return {source:this.source,selection:[...this.selection]}; }
  edit(source, selection=[0,0]) {
    if(source===this.source) {this.selection=selection;return;}
    if(!this.composing) {this.undoStack.push(this.snapshot());this.redoStack=[];}
    this.source=source;this.selection=selection;this.serial++;
  }
  beginComposition() { if(!this.composing) {this.compositionBase=this.snapshot();this.composing=true;} }
  endComposition() {
    if(!this.composing)return;this.composing=false;
    if(this.compositionBase.source!==this.source) {this.undoStack.push(this.compositionBase);this.redoStack=[];}
  }
  move(from,to) {if(this.composing||!from.length)return false;to.push(this.snapshot());const s=from.pop();this.source=s.source;this.selection=s.selection;this.serial++;return true;}
  undo(){return this.move(this.undoStack,this.redoStack);}
  redo(){return this.move(this.redoStack,this.undoStack);}
  accept(result){return result.serial===this.serial&&result.source===this.source;}
  replace(from,to,text,serial){if(serial!==this.serial||from<0||to<from||to>this.source.length)throw Error('Stale source range');this.edit(this.source.slice(0,from)+text+this.source.slice(to),[from+text.length,from+text.length]);}
}
