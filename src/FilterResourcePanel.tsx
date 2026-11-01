import ResourceCanvas from "./ResourceCanvas";
import {useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {open} from '@tauri-apps/plugin-dialog';
import type {FilterResource,Shape,ImageAsset} from './types';
import N from './DraftNumber';
import {identity} from './groups';
import {absoluteContours,normalizePath} from './paths';
import ImagePanel from './ImagePanel';
import PaintEditor from './PaintEditor';
import ClipPanel from './ClipPanel';
import MaskPanel from './MaskPanel';
import FilterPanel from './FilterPanel';
type Key=string|number;
function read<T>(root:unknown,path:Key[]):T{let v=root;for(const k of path)v=(v as Record<Key,unknown>)[k];return v as T;}
function entries(shapes:Shape[],path:Key[]=[],depth=0):{shape:Shape;path:Key[];depth:number}[]{return shapes.flatMap((s,i)=>[{shape:s,path:[...path,i],depth},...(s.children?entries(s.children,[...path,i,'children'],depth+1):[])]);}
const names:Record<Shape['type'],string>={rect:'Rectangle',ellipse:'Ellipse',line:'Ligne',text:'Texte',svg:'Bloc SVG',path:'Tracé',group:'Groupe',image:'Image'};
export default function FilterResourcePanel({resource,width,height,disabled,onChange,onBusy}:{resource:FilterResource;width:number;height:number;disabled:boolean;onChange:(r:FilterResource)=>void;onBusy:(p:boolean)=>void}){
 const [index,setIndex]=useState(0),[nodeIndex,setNodeIndex]=useState(0),[path,setPath]=useState(''),[fit,setFit]=useState('meet'),[align,setAlign]=useState('xMidYMid'),[preview,setPreview]=useState(''),[error,setError]=useState(''),[pending,setPending]=useState(false);
 const alive=useRef(true);useEffect(()=>{alive.current=true;return()=>{alive.current=false;onBusy(false);};},[onBusy]);
 const signature=JSON.stringify(resource);
 useEffect(()=>{let current=true;setPreview('');setError('');const timer=setTimeout(()=>{invoke<string>('filter_resource_preview',{resource}).then(src=>{if(current)setPreview(src);}).catch(e=>{if(current)setError(String(e));});},140);return()=>{current=false;clearTimeout(timer);};},[signature]);
 const items=entries(resource.children),entry=items[Math.min(index,items.length-1)],child=entry?.shape;
 function mutate(fn:(r:FilterResource)=>void){const next=structuredClone(resource);fn(next);onChange(next);setError('');}
 function patch(value:Partial<Shape>){if(entry)mutate(r=>Object.assign(read<Shape>(r.children,entry.path),value));}
 async function load(){setPending(true);onBusy(true);setError('');try{const asset=await invoke<ImageAsset>('embedded_image_file',{path});const next=await invoke<FilterResource>('filter_resource_image',{asset,width,height,fit,align});if(alive.current){onChange(next);setIndex(0);setNodeIndex(0);}}catch(e){if(alive.current)setError(String(e));}finally{if(alive.current){setPending(false);onBusy(false);}}}
 async function decompose(){if(!child)return;setPending(true);onBusy(true);setError('');try{const shapes=await invoke<Shape[]>('decompose_svg_shape',{shape:child});if(alive.current)mutate(r=>read<Shape[]>(r.children,entry.path.slice(0,-1)).splice(entry.path.at(-1)as number,1,...shapes));}catch(e){if(alive.current)setError(String(e));}finally{if(alive.current){setPending(false);onBusy(false);}}}
 const contours=child?.type==='path'?absoluteContours(child):[],nodes=contours.flatMap((c,ci)=>c.nodes.map((n,ni)=>({n,ci,ni}))),node=nodes[Math.min(nodeIndex,nodes.length-1)];
 function editNode(key:'point'|'incoming'|'outgoing',axis:0|1,value:number){if(!child||!node)return;const cs=structuredClone(contours);cs[node.ci].nodes[node.ni][key][axis]=value;patch(normalizePath(child,cs));}
 return <section className="filter-resource-panel"><h3>Source de l’image du filtre</h3>
  {preview&&<div className="filter-preview"><img alt="Aperçu de la source du filtre" src={preview}/></div>}
  <p className="hint">Cette source est intégrée au projet. Son contenu reste indépendant des formes du dessin. Les modifications restent en brouillon jusqu’à Appliquer les effets SVG.</p>
  <fieldset disabled={disabled||pending}><ResourceCanvas label="source du filtre" shapes={resource.children} width={width} height={height} onChange={children=>onChange({...resource,children})}/>
   <label>Fichier de source<input aria-label="Fichier de source du filtre" value={path} onChange={e=>setPath(e.target.value)}/></label>
   <label>Ajustement<select aria-label="Ajustement de la source" value={fit} onChange={e=>setFit(e.target.value)}><option value="meet">Contenir toute l’image</option><option value="slice">Remplir et recadrer</option><option value="none">Étirer</option></select></label>
   <label>Alignement<select aria-label="Alignement de la source" value={align} onChange={e=>setAlign(e.target.value)}>{[['xMinYMin','Haut gauche'],['xMidYMin','Haut centre'],['xMaxYMin','Haut droite'],['xMinYMid','Milieu gauche'],['xMidYMid','Centre'],['xMaxYMid','Milieu droite'],['xMinYMax','Bas gauche'],['xMidYMax','Bas centre'],['xMaxYMax','Bas droite']].map(([v,l])=><option key={v} value={v}>{l}</option>)}</select></label>
   <div className="button-row"><button onClick={async()=>{const value=await open({multiple:false,filters:[{name:'Source intégrée',extensions:['png','jpg','jpeg','gif','webp','svg']}]});if(alive.current&&typeof value==='string')setPath(value);}}>Choisir une source…</button><button disabled={!path.trim()} onClick={()=>void load()}>Charger la source</button></div>
   <p className="hint">L’ajustement et l’alignement s’appliquent au prochain chargement. Charger une source remplace le contenu de ce brouillon.</p>
   <label>Contenu<select aria-label="Contenu de la source du filtre" value={Math.min(index,items.length-1)} onChange={e=>{setIndex(+e.target.value);setNodeIndex(0);setError('');}}>{items.map((s,i)=><option key={JSON.stringify(s.path)} value={i}>{'　'.repeat(s.depth)}{names[s.shape.type]} {i+1}</option>)}</select></label>
   <div className="button-row">{['rect','ellipse'].map(type=><button key={type} onClick={()=>mutate(r=>r.children.push({id:0,type:type as Shape['type'],radius:0,x:0,y:0,w:Math.max(1,width/2),h:Math.max(1,height/2),rotation:0,fill:'#ffffff',stroke:null,strokeWidth:0,opacity:1}))}>{type==='rect'?'Ajouter un rectangle à la source':'Ajouter une ellipse à la source'}</button>)}</div>
   {child&&<section key={JSON.stringify(entry.path)} className="filter-source-content">
    <div className="field-grid">{(['x','y','w','h','rotation','opacity']as const).map((key,i)=><N key={key} label={`Source filtre ${['X','Y','Largeur','Hauteur','Rotation','Opacité'][i]}`} value={child[key]} onChange={v=>patch({[key]:v})}/>)}</div>
    {!['group','image','svg'].includes(child.type)&&<><label>Fond<input aria-label="Fond de la source" value={child.fill??''} onChange={e=>patch({fill:e.target.value||null,fillGradient:null,fillPattern:null})}/></label><label>Trait<input aria-label="Trait de la source" value={child.stroke??''} onChange={e=>patch({stroke:e.target.value||null,strokeGradient:null,strokePattern:null})}/></label><N label="Épaisseur du trait de source" value={child.strokeWidth} onChange={v=>patch({strokeWidth:v})}/><PaintEditor shape={child} disabled={disabled} onChange={patch}/></>}
    {child.type==='image'&&<ImagePanel shape={child} disabled={disabled} onChange={patch} action={async()=>decompose()} onPendingChange={onBusy} draft/>}
    {child.type==='svg'&&<button onClick={()=>void decompose()}>Décomposer le SVG de la source</button>}
    {child.type==='group'&&<><label>Fusion de la source<select aria-label="Fusion du groupe source" value={child.groupStyle?.blend??'normal'} onChange={e=>patch({groupStyle:{blend:e.target.value,isolate:child.groupStyle?.isolate??false}})}>{['normal','multiply','screen','overlay','darken','lighten','color-dodge','color-burn','hard-light','soft-light','difference','exclusion','hue','saturation','color','luminosity','plus-lighter'].map(b=><option key={b} value={b}>{b}</option>)}</select></label><label><input type="checkbox" checked={child.groupStyle?.isolate??false} onChange={e=>patch({groupStyle:{blend:child.groupStyle?.blend??'normal',isolate:e.target.checked}})}/>Isoler la source</label><ClipPanel shape={child} disabled={disabled} onChange={patch}/><MaskPanel shape={child} disabled={disabled} onChange={patch}/><FilterPanel shape={child} disabled={disabled} onChange={patch}/></>}
    {child.type==='text'&&<><label>Texte<input aria-label="Texte de la source" value={child.text} onChange={e=>patch({text:e.target.value})}/></label><label>Police<input aria-label="Police de la source" value={child.fontFamily} onChange={e=>patch({fontFamily:e.target.value})}/></label><N label="Corps du texte source" value={child.fontSize!} onChange={v=>patch({fontSize:v})}/></>}
    {child.type==='path'&&<><label>Remplissage<select aria-label="Règle de la source" value={child.evenOdd?'evenodd':'nonzero'} onChange={e=>patch({evenOdd:e.target.value==='evenodd'})}><option value="nonzero">Non nul</option><option value="evenodd">Pair-impair</option></select></label><details><summary>Nœuds de la source</summary><select aria-label="Nœud de la source" value={Math.min(nodeIndex,nodes.length-1)} onChange={e=>setNodeIndex(+e.target.value)}>{nodes.map((n,i)=><option key={i} value={i}>Contour {n.ci+1} · point {n.ni+1}</option>)}</select>{node&&<div className="field-grid">{(['point','incoming','outgoing']as const).flatMap((key,i)=>([0,1]as const).map(axis=><N key={`${key}${axis}`} label={`Nœud source ${['Ancre','Entrée','Sortie'][i]} ${axis===0?'X':'Y'}`} value={node.n[key][axis]} onChange={v=>editNode(key,axis,v)}/>))}</div>}</details></>}
    <details><summary>Matrice de la source</summary><div className="field-grid">{(child.transform??identity).map((v,i)=><N key={i} label={`Matrice source ${i+1}`} value={v} onChange={v=>patch({transform:(child.transform??identity).map((n,j)=>j===i?v:n)as Shape['transform']})}/>)}</div><button onClick={()=>patch({transform:null})}>Réinitialiser la matrice source</button></details>
    <div className="button-row">{[-1,1].map(delta=>{const peers=read<Shape[]>(resource.children,entry.path.slice(0,-1)),at=entry.path.at(-1)as number;return<button key={delta} disabled={at+delta<0||at+delta>=peers.length} onClick={()=>{mutate(r=>{const peers=read<Shape[]>(r.children,entry.path.slice(0,-1));[peers[at],peers[at+delta]]=[peers[at+delta],peers[at]];});setIndex(0);}}>{delta<0?'Reculer la source':'Avancer la source'}</button>;})}<button disabled={read<Shape[]>(resource.children,entry.path.slice(0,-1)).length===1} onClick={()=>{mutate(r=>read<Shape[]>(r.children,entry.path.slice(0,-1)).splice(entry.path.at(-1)as number,1));setIndex(0);}}>Retirer ce contenu de source</button></div>
   </section>}
  </fieldset>{error&&<p role="alert">{error}</p>}
 </section>;
}
