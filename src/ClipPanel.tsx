import ResourceCanvas from "./ResourceCanvas";
import {useEffect,useState} from "react";
import type {Shape,VectorClip} from "./types";
import {absoluteContours,normalizePath} from "./paths";
import {identity} from "./groups";
type Key=string|number;
function read<T>(root:unknown,path:Key[]):T {let value=root;for(const key of path)value=(value as Record<Key,unknown>)[key];return value as T;}
function options(clip:VectorClip,path:Key[]=[],label="Découpe"): {path:Key[];label:string}[]{
  const out=[{path,label}];if(clip.intersect)out.push(...options(clip.intersect,[...path,'intersect'],`${label} · intersection`));
  function walk(shapes:Shape[],at:Key[]){shapes.forEach((s,i)=>{if(s.clip)out.push(...options(s.clip,[...at,i,'clip'],`${label} · groupe ${i+1}`));if(s.children)walk(s.children,[...at,i,'children']);});}
  walk(clip.paths,[...path,'paths']);return out;
}
function shapes(list:Shape[],path:Key[]=[]):{shape:Shape;path:Key[];depth:number}[]{return list.flatMap((shape,i)=>[{shape,path:[...path,i],depth:path.length/2},...(shape.children?shapes(shape.children,[...path,i,'children']):[])]);}
function initial(shape:Shape,ellipse=false):VectorClip {
  const p=(point:[number,number],incoming=point,outgoing=point)=>({point,incoming,outgoing});const k=.2761424;
  const nodes=ellipse?[p([.5,0],[.5-k,0],[.5+k,0]),p([1,.5],[1,.5-k],[1,.5+k]),p([.5,1],[.5+k,1],[.5-k,1]),p([0,.5],[0,.5+k],[0,.5-k])]:[[0,0],[1,0],[1,1],[0,1]].map(v=>p(v as [number,number]));
  return{transform:[...identity],paths:[{id:0,type:'path',x:0,y:0,w:shape.intrinsicW!,h:shape.intrinsicH!,rotation:0,fill:'#000000',stroke:null,strokeWidth:0,opacity:1,evenOdd:false,contours:[{closed:true,nodes}]}]};
}
export default function ClipPanel({shape,disabled,onChange}:{shape:Shape;disabled:boolean;onChange:(patch:Partial<Shape>)=>void}){
  const [draft,setDraft]=useState<VectorClip|null>(shape.clip??null),[step,setStep]=useState(0),[index,setIndex]=useState(0),[nodeIndex,setNodeIndex]=useState(0);
  const signature=JSON.stringify(shape.clip);useEffect(()=>{setDraft(shape.clip?structuredClone(shape.clip):null);setStep(0);setIndex(0);setNodeIndex(0);},[signature,shape.id]);
  const choices=draft?options(draft):[];const selected=choices[Math.min(step,choices.length-1)];const clip=draft&&selected?read<VectorClip>(draft,selected.path):null;
  const entries=clip?shapes(clip.paths):[];const entry=entries[Math.min(index,entries.length-1)];const geometry=entry?.shape;
  function patch<T extends object>(path:Key[],value:Partial<T>){if(!draft)return;const next=structuredClone(draft);Object.assign(read<object>(next,path),value);setDraft(next);}
  const geometryPath=[...(selected?.path??[]),'paths',...(entry?.path??[])];
  const cs=geometry?.type==='path'?absoluteContours(geometry):[];const nodes=cs.flatMap((c,ci)=>c.nodes.map((n,ni)=>({n,ci,ni})));const node=nodes[Math.min(nodeIndex,nodes.length-1)];
  function editNode(key:'point'|'incoming'|'outgoing',axis:0|1,value:number){if(!geometry||!node)return;const contours=structuredClone(cs);contours[node.ci].nodes[node.ni][key][axis]=value;patch<Shape>(geometryPath,normalizePath(geometry,contours));}
  return <details className="clip-panel"><summary>Découpe vectorielle</summary><fieldset disabled={disabled}>
    <div className="button-row"><button onClick={()=>{setDraft(initial(shape));setStep(0);setIndex(0);}}>Rectangle</button><button onClick={()=>{setDraft(initial(shape,true));setStep(0);setIndex(0);}}>Ellipse</button></div>
    {draft&&clip&&geometry&&<><ResourceCanvas label="découpe" shapes={clip.paths} width={shape.intrinsicW??shape.w} height={shape.intrinsicH??shape.h} onChange={paths=>patch<VectorClip>(selected.path,{paths})}/>
      <label>Étape<select aria-label="Étape de découpe" value={step} onChange={e=>{setStep(+e.target.value);setIndex(0);setNodeIndex(0);}}>{choices.map((c,i)=><option key={i} value={i}>{c.label}</option>)}</select></label>
      <label>Géométrie<select aria-label="Géométrie de découpe" value={Math.min(index,entries.length-1)} onChange={e=>{setIndex(+e.target.value);setNodeIndex(0);}}>{entries.map((s,i)=><option key={i} value={i}>{'　'.repeat(s.depth)}{s.shape.type==='group'?'Groupe':'Tracé'} {i+1}</option>)}</select></label>
      <div className="field-grid">{(['x','y','w','h','rotation']as const).map((key,i)=><label key={key}>{['X','Y','Largeur','Hauteur','Rotation'][i]}<input aria-label={`Découpe ${['X','Y','Largeur','Hauteur','Rotation'][i]}`} type="number" step="any" value={geometry[key]} onChange={e=>{if(Number.isFinite(e.target.valueAsNumber))patch<Shape>(geometryPath,{[key]:e.target.valueAsNumber});}}/></label>)}</div>
      {geometry.type==='path'&&<><label>Remplissage<select aria-label="Règle de découpe" value={geometry.evenOdd?'evenodd':'nonzero'} onChange={e=>patch<Shape>(geometryPath,{evenOdd:e.target.value==='evenodd'})}><option value="nonzero">Non nul</option><option value="evenodd">Pair-impair (trous)</option></select></label>
      <details><summary>Nœuds de la découpe</summary><label>Nœud<select aria-label="Nœud de découpe" value={Math.min(nodeIndex,nodes.length-1)} onChange={e=>setNodeIndex(+e.target.value)}>{nodes.map((n,i)=><option key={i} value={i}>Contour {n.ci+1} · point {n.ni+1}</option>)}</select></label>
      {node&&<div className="field-grid">{(['point','incoming','outgoing']as const).flatMap((key,i)=>([0,1]as const).map(axis=><label key={`${key}${axis}`}>{['Ancre','Entrée','Sortie'][i]} {axis===0?'X':'Y'}<input aria-label={`Découpe ${key} ${axis===0?'X':'Y'}`} type="number" step="any" value={node.n[key][axis]} onChange={e=>{if(Number.isFinite(e.target.valueAsNumber))editNode(key,axis,e.target.valueAsNumber);}}/></label>))}</div>}</details></>}
      <details><summary>Transformation de la découpe</summary><div className="field-grid">{clip.transform.map((v,i)=><label key={i}>{['A','B','C','D','X','Y'][i]}<input aria-label={`Matrice découpe ${i+1}`} type="number" step="any" value={v} onChange={e=>{if(Number.isFinite(e.target.valueAsNumber))patch<VectorClip>(selected.path,{transform:clip.transform.map((n,j)=>j===i?e.target.valueAsNumber:n)as VectorClip['transform']});}}/></label>)}</div><button onClick={()=>patch<VectorClip>(selected.path,{transform:[...identity]})}>Réinitialiser la matrice</button></details>
      <button onClick={()=>onChange({clip:draft})}>Appliquer la découpe</button>
    </>}
    <div className="button-row"><button onClick={()=>setDraft(shape.clip?structuredClone(shape.clip):null)}>Annuler les réglages de découpe</button><button disabled={!shape.clip} onClick={()=>onChange({clip:null})}>Retirer la découpe</button></div>
    <p className="hint">Les réglages restent en brouillon. La découpe masque le contenu sans supprimer ses pixels ni ses formes. Ses coordonnées sont locales au groupe ; Appliquer crée une étape annulable.</p>
  </fieldset></details>;
}
