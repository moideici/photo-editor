import ResourceCanvas from "./ResourceCanvas";
import type { Shape, SvgPattern } from "./types";
import {identity} from "./groups";
import {useEffect,useState} from "react";
import {invoke} from "@tauri-apps/api/core";
export function initialPattern(kind="checker"):SvgPattern {
  const shape=(type:Shape["type"],x:number,y:number,w:number,h:number,fill:string):Shape=>({id:0,type,radius:0,x,y,w,h,rotation:0,fill,stroke:null,strokeWidth:0,opacity:1});
  let children=[shape("rect",0,0,20,20,"#ffffff"),shape("rect",0,0,10,10,"#304060"),shape("rect",10,10,10,10,"#304060")];
  if(kind==="stripes")children=[shape("rect",0,0,20,20,"#ffffff"),shape("rect",0,0,8,20,"#304060")];
  if(kind==="dots")children=[shape("ellipse",5,5,10,10,"#304060")];
  return {units:"userSpaceOnUse",transform:[...identity],x:0,y:0,width:20,height:20,contentWidth:20,contentHeight:20,children};
}
export default function PatternFields({pattern,onChange}:{pattern:SvgPattern;onChange:(v:SvgPattern)=>void}) {
  const [preview,setPreview]=useState("");
  const [previewError,setPreviewError]=useState("");
  const [pending,setPending]=useState(false);
  const signature=JSON.stringify(pattern);
  useEffect(()=>{let current=true;setPending(true);setPreview("");setPreviewError("");const timer=setTimeout(()=>{invoke<string>("pattern_preview",{pattern}).then(src=>{if(current)setPreview(src);}).catch(e=>{if(current)setPreviewError(String(e));}).finally(()=>{if(current)setPending(false);});},120);return()=>{current=false;clearTimeout(timer);};},[signature]);
  const patch=(p:Partial<SvgPattern>)=>onChange({...pattern,...p});
  const child=(index:number,p:Partial<Shape>)=>patch({children:pattern.children.map((s,i)=>i===index?{...s,...p}:s)});
  return <>
    <div className="pattern-preview" aria-busy={pending}>{preview&&<img alt="Contenu d’un carreau du motif" src={preview}/ >}{pending&&<span>Calcul du carreau…</span>}</div>
    <p className="hint">Aperçu du contenu d’un carreau, avant sa matrice et sa répétition.</p>
    {previewError&&<p role="alert">{previewError}</p>}
    <label>Modèle<select aria-label="Modèle du motif" value="custom" onChange={e=>onChange({...initialPattern(e.target.value),units:pattern.units,transform:pattern.transform})}><option value="custom" disabled>Personnalisé</option><option value="checker">Damier</option><option value="stripes">Rayures</option><option value="dots">Pois transparents</option></select></label>
    <div className="field-grid">{([["x","Origine X"],["y","Origine Y"],["width","Largeur du carreau"],["height","Hauteur du carreau"],["contentWidth","Largeur du contenu"],["contentHeight","Hauteur du contenu"]] as const).map(([key,label])=><label key={key}>{label}<input aria-label={`Motif ${label}`} type="number" step="any" value={pattern[key]} onChange={e=>{if(Number.isFinite(e.target.valueAsNumber))patch({[key]:e.target.valueAsNumber});}}/></label>)}</div>
    <p className="hint">Le carreau se répète. Le contenu est mis à son échelle et découpé à ses bords. Les formes sont superposées dans l’ordre de la liste.</p>
    <ResourceCanvas label="motif" shapes={pattern.children} width={pattern.contentWidth} height={pattern.contentHeight} onChange={children=>patch({children})}/><div className="pattern-children">{pattern.children.map((s,i)=><details key={i} open={i===0}>
      <summary>Élément {i+1} — {s.type}</summary>
      <div className="field-grid">{([['x','X'],['y','Y'],['w','Largeur'],['h','Hauteur'],['rotation','Rotation'],['opacity','Opacité']] as const).map(([key,label])=><label key={key}>{label}<input aria-label={`Motif élément ${i+1} ${label}`} type="number" step="any" value={s[key]} onChange={e=>{if(Number.isFinite(e.target.valueAsNumber))child(i,{[key]:e.target.valueAsNumber});}}/></label>)}</div>
      <label><input type="checkbox" checked={s.fill!==null || !!s.fillGradient} onChange={e=>child(i,{fill:e.target.checked?'#304060':null,fillGradient:null,fillPattern:null})}/>Fond</label>
      <input type="color" aria-label={`Motif élément ${i+1} couleur`} value={/^#[0-9a-f]{6}$/i.test(s.fill??'')?s.fill!:'#304060'} onChange={e=>child(i,{fill:e.target.value,fillGradient:null,fillPattern:null})}/>
      {s.fillGradient&&<p className="hint">Fond en dégradé importé conservé ; choisir une couleur le remplace.</p>}
      <div className="button-row"><button disabled={i===0} onClick={()=>{const children=[...pattern.children];[children[i-1],children[i]]=[children[i],children[i-1]];patch({children});}}>Reculer</button><button disabled={i===pattern.children.length-1} onClick={()=>{const children=[...pattern.children];[children[i+1],children[i]]=[children[i],children[i+1]];patch({children});}}>Avancer</button><button aria-label={`Retirer élément ${i+1}`} disabled={pattern.children.length===1} onClick={()=>patch({children:pattern.children.filter((_,j)=>j!==i)})}>Retirer</button></div>
    </details>)}</div>
    <div className="button-row">{([['rect','Rectangle'],['ellipse','Ellipse']] as const).map(([type,label])=><button key={type} disabled={pattern.children.length>=200} onClick={()=>patch({children:[...pattern.children,{id:0,type,radius:0,x:0,y:0,w:pattern.contentWidth/2,h:pattern.contentHeight/2,rotation:0,fill:'#304060',stroke:null,strokeWidth:0,opacity:1}]})}>Ajouter {label.toLowerCase()}</button>)}</div>
    <button disabled={pattern.children.length>=200} onClick={()=>patch({children:[...pattern.children,{id:0,type:'path',x:0,y:0,w:pattern.contentWidth,h:pattern.contentHeight,rotation:0,fill:'#304060',stroke:null,strokeWidth:0,opacity:1,evenOdd:true,contours:[{closed:true,nodes:[[.1,.1],[.9,.2],[.5,.9]].map(p=>({point:p as [number,number],incoming:p as [number,number],outgoing:p as [number,number]}))}]}]})}>Ajouter un tracé au motif</button><details><summary>Transformation du motif</summary><div className="field-grid">{pattern.transform.map((v,i)=><label key={i}>{['A','B','C','D','X','Y'][i]}<input aria-label={`Matrice motif ${i+1}`} type="number" step="any" value={v} onChange={e=>{if(Number.isFinite(e.target.valueAsNumber))patch({transform:pattern.transform.map((x,j)=>i===j?e.target.valueAsNumber:x) as SvgPattern['transform']});}}/></label>)}</div><button onClick={()=>patch({transform:[...identity]})}>Réinitialiser la transformation</button></details>
  </>;
}
