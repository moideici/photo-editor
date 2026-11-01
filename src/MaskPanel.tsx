import ResourceCanvas from "./ResourceCanvas";
import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import type {Shape,VectorMask} from './types';
import {identity} from './groups';
import {absoluteContours,normalizePath} from './paths';
import N from './DraftNumber';
import PaintEditor from './PaintEditor';
import ImagePanel from './ImagePanel';
import FilterPanel from './FilterPanel';
type Key=string|number;
function read<T>(root:unknown,path:Key[]):T {let v=root;for(const key of path)v=(v as Record<Key,unknown>)[key];return v as T;}
function stages(mask:VectorMask,path:Key[]=[],label='Masque'):{path:Key[];label:string}[]{
 const out=[{path,label}];if(mask.intersect)out.push(...stages(mask.intersect,[...path,'intersect'],`${label} · intersection`));
 function walk(shapes:Shape[],at:Key[]){shapes.forEach((s,i)=>{if(s.mask)out.push(...stages(s.mask,[...at,i,'mask'],`${label} · groupe ${i+1}`));if(s.children)walk(s.children,[...at,i,'children']);});}walk(mask.children,[...path,'children']);return out;
}
function entries(shapes:Shape[],path:Key[]=[],depth=0):{shape:Shape;path:Key[];depth:number}[]{return shapes.flatMap((s,i)=>[{shape:s,path:[...path,i],depth},...(s.children?entries(s.children,[...path,i,'children'],depth+1):[])]);}
function rectangle(x:number,y:number,w:number,h:number):Shape{return{id:0,type:'rect',radius:0,x,y,w,h,rotation:0,fill:'#ffffff',stroke:null,strokeWidth:0,opacity:1};}
function initial(shape:Shape):VectorMask{return{transform:[...identity],region:[0,0,shape.intrinsicW!,shape.intrinsicH!],mode:'luminance',children:[rectangle(0,0,shape.intrinsicW!,shape.intrinsicH!)]};}
const blends=['normal','multiply','screen','overlay','darken','lighten','color-dodge','color-burn','hard-light','soft-light','difference','exclusion','hue','saturation','color','luminosity','plus-lighter'];
const blendNames=['Normal','Produit','Écran','Incrustation','Assombrir','Éclaircir','Densité couleur −','Densité couleur +','Lumière dure','Lumière douce','Différence','Exclusion','Teinte','Saturation','Couleur','Luminosité','Addition'];
const shapeNames:Record<Shape['type'],string>={rect:'Rectangle',ellipse:'Ellipse',line:'Ligne',text:'Texte',svg:'Bloc SVG',path:'Tracé',group:'Groupe',image:'Image'};
export default function MaskPanel({shape,disabled,onChange}:{shape:Shape;disabled:boolean;onChange:(p:Partial<Shape>)=>void}){
 const [draft,setDraft]=useState<VectorMask|null>(shape.mask??null),[step,setStep]=useState(0),[index,setIndex]=useState(0),[nodeIndex,setNodeIndex]=useState(0),[preview,setPreview]=useState(''),[error,setError]=useState(''),[pending,setPending]=useState(false),[decomposing,setDecomposing]=useState(false);
 const signature=JSON.stringify(shape.mask);useEffect(()=>{setDraft(shape.mask?structuredClone(shape.mask):null);setStep(0);setIndex(0);setNodeIndex(0);setError('');},[signature,shape.id]);
 const draftSignature=JSON.stringify(draft);
 useEffect(()=>{if(!draft){setPreview('');setPending(false);return;}let current=true;setPending(true);setPreview('');setError('');const timer=setTimeout(()=>{invoke<string>('mask_preview',{mask:draft}).then(src=>{if(current)setPreview(src);}).catch(e=>{if(current)setError(String(e));}).finally(()=>{if(current)setPending(false);});},120);return()=>{current=false;clearTimeout(timer);};},[draftSignature]);
 const choices=draft?stages(draft):[],selected=choices[Math.min(step,choices.length-1)],mask=draft&&selected?read<VectorMask>(draft,selected.path):null,items=mask?entries(mask.children):[],entry=items[Math.min(index,items.length-1)],child=entry?.shape,childPath=[...(selected?.path??[]),'children',...(entry?.path??[])];
 function patch<T extends object>(path:Key[],value:Partial<T>){setDraft(old=>{if(!old)return old;const next=structuredClone(old);Object.assign(read<object>(next,path),value);return next;});}
 const patchChild=(value:Partial<Shape>)=>patch<Shape>(childPath,value);
 const cs=child?.type==='path'?absoluteContours(child):[],nodes=cs.flatMap((c,ci)=>c.nodes.map((n,ni)=>({n,ci,ni}))),node=nodes[Math.min(nodeIndex,nodes.length-1)];
 function editNode(key:'point'|'incoming'|'outgoing',axis:0|1,value:number){if(!child||!node)return;const contours=structuredClone(cs);contours[node.ci].nodes[node.ni][key][axis]=value;patchChild(normalizePath(child,contours));}
 async function decompose(){if(!child)return;setDecomposing(true);setError('');try{const result=await invoke<Shape[]>('decompose_svg_shape',{shape:child});setDraft(old=>{if(!old)return old;const next=structuredClone(old);const parent=read<Shape[]>(next,childPath.slice(0,-1));parent.splice(childPath.at(-1)as number,1,...result);return next;});}catch(e){setError(String(e));}finally{setDecomposing(false);}}
 return <details className="mask-panel"><summary>Masque vectoriel</summary><fieldset disabled={disabled||decomposing}>
  <button onClick={()=>{setDraft(initial(shape));setStep(0);setIndex(0);}}>Créer un masque blanc</button>
  {draft&&mask&&<><ResourceCanvas label="masque" shapes={mask.children} width={shape.intrinsicW??shape.w} height={shape.intrinsicH??shape.h} onChange={children=>patch<VectorMask>(selected.path,{children})}/>
   <div className="mask-preview" aria-busy={pending}>{preview&&<img alt="Couverture du masque" src={preview}/ >}{pending&&<span>Calcul du masque…</span>}</div>
   <p className="hint">Blanc : visible ; transparent : caché. Cet aperçu montre la couverture du masque. Les réglages restent en brouillon jusqu’à Appliquer.</p>
   <label>Étape<select aria-label="Étape du masque" value={step} onChange={e=>{setStep(+e.target.value);setIndex(0);setNodeIndex(0);}}>{choices.map((c,i)=><option key={i} value={i}>{c.label}</option>)}</select></label>
   <label>Mode<select aria-label="Mode du masque" value={mask.mode} onChange={e=>patch<VectorMask>(selected.path,{mode:e.target.value as VectorMask['mode']})}><option value="luminance">Luminance et transparence</option><option value="alpha">Transparence seule</option></select></label>
   <details><summary>Région du masque</summary><div className="field-grid">{mask.region.map((v,i)=><N key={i} label={`Région masque ${['X','Y','Largeur','Hauteur'][i]}`} value={v} onChange={v=>patch<VectorMask>(selected.path,{region:mask.region.map((n,j)=>j===i?v:n)as VectorMask['region']})}/>)}</div></details>
   <label>Contenu<select aria-label="Forme du masque" value={Math.min(index,items.length-1)} onChange={e=>{setIndex(+e.target.value);setNodeIndex(0);}}>{items.map((s,i)=><option key={i} value={i}>{'　'.repeat(s.depth)}{shapeNames[s.shape.type]} {i+1}</option>)}{!items.length&&<option value={-1}>Masque vide</option>}</select></label>
   <div className="button-row"><button onClick={()=>patch<VectorMask>(selected.path,{children:[...mask.children,rectangle(mask.region[0],mask.region[1],mask.region[2]/2,mask.region[3]/2)]})}>Ajouter un rectangle au masque</button><button onClick={()=>patch<VectorMask>(selected.path,{children:[...mask.children,{...rectangle(mask.region[0],mask.region[1],mask.region[2]/2,mask.region[3]/2),type:'ellipse'}]})}>Ajouter une ellipse au masque</button></div>
   {child&&<section className="mask-content-editor" key={JSON.stringify(childPath)}>
    <div className="field-grid">{(['x','y','w','h','rotation','opacity']as const).map((key,i)=><N key={key} label={`Contenu masque ${['X','Y','Largeur','Hauteur','Rotation','Opacité'][i]}`} value={child[key]} onChange={v=>patchChild({[key]:v})}/>)}</div>
    {!['image','group','svg'].includes(child.type)&&<>
     <label>Fond<input aria-label="Fond du contenu du masque" value={child.fill??''} placeholder="Aucun" onChange={e=>patchChild({fill:e.target.value||null,fillGradient:null,fillPattern:null})}/></label>
     <label>Trait<input aria-label="Trait du contenu du masque" value={child.stroke??''} placeholder="Aucun" onChange={e=>patchChild({stroke:e.target.value||null,strokeGradient:null,strokePattern:null})}/></label>
     <N label="Épaisseur du trait du masque" value={child.strokeWidth} onChange={v=>patchChild({strokeWidth:v})}/>
     <PaintEditor shape={child} disabled={disabled} onChange={patchChild}/><p className="hint">Appliquer la peinture actualise le brouillon du masque ; valider ensuite le masque.</p>
    </>}
    {child.type==='group'&&<><label>Fusion<select aria-label="Fusion du contenu du masque" value={child.groupStyle?.blend??'normal'} onChange={e=>patchChild({groupStyle:{blend:e.target.value,isolate:child.groupStyle?.isolate??false}})}>{blends.map((b,i)=><option key={b} value={b}>{blendNames[i]}</option>)}</select></label><label><input aria-label="Isoler le contenu du masque" type="checkbox" checked={child.groupStyle?.isolate??false} onChange={e=>patchChild({groupStyle:{blend:child.groupStyle?.blend??'normal',isolate:e.target.checked}})}/>Isoler la fusion</label></>}
    {child.type==='group'&&<FilterPanel shape={child} disabled={disabled} onChange={patchChild}/>}
    {child.type==='image'&&<ImagePanel shape={child} disabled={disabled} onChange={patchChild} action={async()=>decompose()}/>}
    {child.type==='svg'&&<button onClick={()=>void decompose()}>Décomposer le SVG du masque</button>}
    {child.type==='text'&&<><label>Texte<input aria-label="Texte du masque" value={child.text} onChange={e=>patchChild({text:e.target.value})}/></label><label>Police<input aria-label="Police du masque" value={child.fontFamily} onChange={e=>patchChild({fontFamily:e.target.value})}/></label><N label="Corps du texte du masque" value={child.fontSize!} onChange={v=>patchChild({fontSize:v})}/></>}
    {child.type==='path'&&<><label>Remplissage<select aria-label="Règle du contenu du masque" value={child.evenOdd?'evenodd':'nonzero'} onChange={e=>patchChild({evenOdd:e.target.value==='evenodd'})}><option value="nonzero">Non nul</option><option value="evenodd">Pair-impair</option></select></label><details><summary>Nœuds du masque</summary><select aria-label="Nœud du masque" value={Math.min(nodeIndex,nodes.length-1)} onChange={e=>setNodeIndex(+e.target.value)}>{nodes.map((n,i)=><option key={i} value={i}>Contour {n.ci+1} · point {n.ni+1}</option>)}</select>{node&&<div className="field-grid">{(['point','incoming','outgoing']as const).flatMap((key,i)=>([0,1]as const).map(axis=><N key={`${key}${axis}`} label={`Nœud masque ${['Ancre','Entrée','Sortie'][i]} ${axis===0?'X':'Y'}`} value={node.n[key][axis]} onChange={v=>editNode(key,axis,v)}/>))}</div>}</details></>}
    <details><summary>Matrice du contenu</summary><div className="field-grid">{(child.transform??identity).map((v,i)=><N key={i} label={`Matrice contenu masque ${i+1}`} value={v} onChange={v=>patchChild({transform:(child.transform??identity).map((n,j)=>j===i?v:n)as VectorMask['transform']})}/>)}</div><button onClick={()=>patchChild({transform:null})}>Réinitialiser la matrice du contenu</button></details>
    <div className="button-row">{[-1,1].map(delta=>{const peers=read<Shape[]>(draft,childPath.slice(0,-1)),at=childPath.at(-1)as number;return<button key={delta} disabled={at+delta<0||at+delta>=peers.length} onClick={()=>{const children=[...peers];[children[at],children[at+delta]]=[children[at+delta],children[at]];setDraft(old=>{if(!old)return old;const next=structuredClone(old);read<Shape[]>(next,childPath.slice(0,-1)).splice(0,peers.length,...children);return next;});setIndex(0);}}>{delta<0?'Reculer le contenu':'Avancer le contenu'}</button>;})}<button onClick={()=>{setDraft(old=>{if(!old)return old;const next=structuredClone(old);read<Shape[]>(next,childPath.slice(0,-1)).splice(childPath.at(-1)as number,1);return next;});setIndex(0);}}>Retirer ce contenu du masque</button></div>
   </section>}
   <details><summary>Transformation du masque</summary><div className="field-grid">{mask.transform.map((v,i)=><N key={i} label={`Matrice masque ${i+1}`} value={v} onChange={v=>patch<VectorMask>(selected.path,{transform:mask.transform.map((n,j)=>j===i?v:n)as VectorMask['transform']})}/>)}</div><button onClick={()=>patch<VectorMask>(selected.path,{transform:[...identity]})}>Réinitialiser la matrice du masque</button></details>
   <button disabled={pending||!!error||draftSignature===signature} onClick={()=>onChange({mask:draft})}>Appliquer le masque</button>
  </>}
  <div className="button-row"><button onClick={()=>{setDraft(shape.mask?structuredClone(shape.mask):null);setStep(0);setIndex(0);setError('');}}>Annuler les réglages du masque</button><button disabled={!shape.mask} onClick={()=>onChange({mask:null})}>Retirer le masque</button></div>
  {error&&<p role="alert">{error}</p>}<p className="hint">Le masque conserve son contenu et celui du groupe. En luminance, le noir cache et le blanc révèle ; en alpha, seule la transparence compte. Retirer le masque est annulable.</p>
 </fieldset></details>;
}
