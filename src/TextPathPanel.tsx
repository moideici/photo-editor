import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import type {Layer,Shape,TextPath} from './types';
import {entries} from './groups';
import N from './DraftNumber';
type Settings=Pick<TextPath,'sourceId'|'contourIndex'|'startOffset'|'percent'|'normalOffset'|'anchor'|'reverse'>;
const settings=(s:Shape):Settings=>({sourceId:s.textPath?.sourceId??null,contourIndex:s.textPath?.contourIndex??0,startOffset:s.textPath?.startOffset??0,percent:s.textPath?.percent??false,normalOffset:s.textPath?.normalOffset??0,anchor:s.textPath?.anchor??'start',reverse:s.textPath?.reverse??false});
export default function TextPathPanel({shape,layer,disabled,onChange}:{shape:Shape;layer:Layer;disabled:boolean;onChange:(p:Partial<Shape>)=>void}){
 const [draft,setDraft]=useState(()=>settings(shape)),[result,setResult]=useState<{shape:Shape;src:string}|null>(null),[error,setError]=useState(''),[pending,setPending]=useState(false);
 const sourceSignature=JSON.stringify(layer.shapes),signature=JSON.stringify(draft),shapeSignature=JSON.stringify(shape);
 useEffect(()=>{setDraft(settings(shape));},[shapeSignature]);
 useEffect(()=>{let current=true;setResult(null);setError('');
  if(draft.sourceId==null&&!shape.textPath){setPending(false);return;}
  setPending(true);const timer=setTimeout(()=>{invoke<{shape:Shape;src:string}>('text_path_layout',{shapes:layer.shapes,shapeId:shape.id,settings:draft}).then(r=>{if(current)setResult(r);}).catch(e=>{if(current)setError(String(e));}).finally(()=>{if(current)setPending(false);});},150);
  return()=>{current=false;clearTimeout(timer);};
 },[sourceSignature,signature]);
 const guides=entries(layer.shapes??[]).filter(({shape:s})=>s.id!==shape.id&&(['path','line','ellipse'].includes(s.type)||(s.type==='rect'&&!s.radius)));
 const guide=guides.find(e=>e.shape.id===draft.sourceId)?.shape,count=guide?.type==='path'?guide.contours?.length??0:1;
 const patch=(p:Partial<Settings>)=>setDraft(d=>({...d,...p}));
 return <details className="text-path-panel"><summary>Texte sur tracé</summary><fieldset disabled={disabled}>
  {result&&<div className="filter-preview"><img alt="Aperçu du texte sur tracé" src={result.src}/></div>}
  {pending&&<p role="status">Calcul de l’aperçu du texte…</p>}
  <label>Chemin<select aria-label="Chemin du texte" value={draft.sourceId??''} onChange={e=>patch({sourceId:e.target.value?+e.target.value:null,contourIndex:0})}><option value="">{shape.textPath?'Courbe détachée':'Choisir un chemin'}</option>{guides.map(({shape:s,depth})=><option key={s.id} value={s.id}>{'　'.repeat(depth)}{s.type==='path'?'Tracé':s.type==='line'?'Ligne':s.type==='ellipse'?'Ellipse':'Rectangle'} #{s.id}</option>)}</select></label>
  {draft.sourceId!=null&&<label>Contour<select aria-label="Contour du chemin du texte" value={draft.contourIndex} onChange={e=>patch({contourIndex:+e.target.value})}>{Array.from({length:count},(_,i)=><option key={i} value={i}>Contour {i+1}</option>)}</select></label>}
  <N label="Départ du texte sur tracé" value={draft.startOffset} onChange={v=>patch({startOffset:v})}/>
  <label>Unité du départ<select aria-label="Unité du départ du texte" value={draft.percent?'percent':'pixels'} onChange={e=>patch({percent:e.target.value==='percent'})}><option value="pixels">Pixels du chemin</option><option value="percent">Pourcentage du chemin</option></select></label>
  <N label="Écart à la ligne du texte" value={draft.normalOffset} onChange={v=>patch({normalOffset:v})}/>
  <label>Alignement<select aria-label="Alignement du texte sur tracé" value={draft.anchor} onChange={e=>patch({anchor:e.target.value as Settings['anchor']})}><option value="start">Début</option><option value="middle">Centre</option><option value="end">Fin</option></select></label>
  <label><input type="checkbox" aria-label="Inverser le sens du texte" checked={draft.reverse} onChange={e=>patch({reverse:e.target.checked})}/>Inverser le sens du chemin</label>
  <div className="button-row"><button disabled={pending||!result} onClick={()=>result&&onChange(result.shape)}>Appliquer le texte sur tracé</button><button disabled={pending||(!shape.textPath&&draft.sourceId==null)} onClick={()=>{setDraft(settings(shape));}}>Annuler les réglages du texte sur tracé</button></div>
  {shape.textPath&&<><button disabled={pending||shape.textPath.sourceId==null} onClick={()=>patch({sourceId:null})}>Détacher le chemin en gardant la courbe</button><button onClick={()=>onChange({textPath:null,w:Math.max(1,(shape.text?.length??1)*(shape.fontSize??48)*.6),h:(shape.fontSize??48)*1.3})}>Revenir au texte droit</button></>}
  <p className="hint">Le texte reste éditable. Le guide lié suit les modifications de sa forme, même dans un groupe. Réglez sa position avec le départ et l’écart ; détachez-le pour déplacer ou transformer le texte indépendamment. Les lettres hors du chemin restent dans le texte mais ne sont pas affichées. Le brouillon ne modifie pas le dessin avant Appliquer.</p>
  {error&&<p role="alert">{error}</p>}
 </fieldset></details>;
}
