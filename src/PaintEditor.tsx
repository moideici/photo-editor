import { useEffect, useState } from "react";
import {invoke} from "@tauri-apps/api/core";
import type { Gradient, Shape, SvgPattern } from "./types";
import PatternFields, { initialPattern } from "./PatternFields";
import { identity, inverse, multiply } from "./groups";

function initial(kind:"linear"|"radial", color:string):Gradient {
  return {geometry:kind==="linear"?{kind,start:[0,0],end:[1,0]}:{kind,center:[.5,.5],focus:[.5,.5],radius:.5,focalRadius:0},units:"objectBoundingBox",transform:[...identity],spread:"pad",stops:[{offset:0,color,opacity:1},{offset:1,color:"#ffffff",opacity:1}]};
}
export default function PaintEditor({shape,disabled,onChange}:{shape:Shape;disabled:boolean;onChange:(patch:Partial<Shape>)=>void}) {
  const [target,setTarget]=useState<"fill"|"stroke">("fill");
  const [error,setError]=useState("");
  const [converting,setConverting]=useState(false);
  const value=target==="fill"?shape.fillGradient:shape.strokeGradient;
  const patternValue=target==="fill"?shape.fillPattern:shape.strokePattern;
  const [pattern,setPattern]=useState<SvgPattern|null>(patternValue??null);
  const [draft,setDraft]=useState<Gradient|null>(value??null);
  const signature=JSON.stringify([value,patternValue]);
  function reset(){setDraft(value?structuredClone(value):null);setPattern(patternValue?structuredClone(patternValue):null);setError("");}
  useEffect(reset,[signature,target,shape.id]);
  const update=(patch:Partial<Gradient>)=>setDraft(d=>d?{...d,...patch}:d);
  const geometry=draft?.geometry;
  const coordinates: [string,number,(value:number)=>void][]=[];
  if(geometry?.kind==="linear") {
    for(const [key,label] of [["start","Départ"],["end","Arrivée"]] as const)
      for(const axis of [0,1] as const)coordinates.push([`${label} ${axis===0?"X":"Y"}`,geometry[key][axis],v=>update({geometry:{...geometry,[key]:geometry[key].map((p,i)=>i===axis?v:p) as [number,number]}})]);
  } else if(geometry?.kind==="radial") {
    for(const [key,label] of [["center","Centre"],["focus","Foyer"]] as const)
      for(const axis of [0,1] as const)coordinates.push([`${label} ${axis===0?"X":"Y"}`,geometry[key][axis],v=>update({geometry:{...geometry,[key]:geometry[key].map((p,i)=>i===axis?v:p) as [number,number]}})]);
    coordinates.push(["Rayon",geometry.radius,v=>update({geometry:{...geometry,radius:v}})],["Rayon du foyer",geometry.focalRadius,v=>update({geometry:{...geometry,focalRadius:v}})]);
  }
  async function units(units:Gradient["units"]){
    if(!draft||draft.units===units)return;
    setConverting(true);setError("");
    try {const [x,y,w,h]=await invoke<[number,number,number,number]>("paint_bounds",{shape});
      const bbox:[number,number,number,number,number,number]=[w,0,0,h,x,y];
      update({units,transform:multiply(units==="userSpaceOnUse"?bbox:inverse(bbox),draft.transform)});
    }catch(e){setError(String(e));}finally{setConverting(false);}
  }
  async function patternUnits(units:SvgPattern["units"]){
    if(!pattern||pattern.units===units)return;
    setConverting(true);setError("");
    try {const [x,y,w,h]=await invoke<[number,number,number,number]>("paint_bounds",{shape});const bbox:Gradient["transform"]=[w,0,0,h,x,y];setPattern({...pattern,units,transform:multiply(units==="userSpaceOnUse"?bbox:inverse(bbox),pattern.transform)});}
    catch(e){setError(String(e));}finally{setConverting(false);}
  }
  function apply(){
    const gradient=draft?{...draft,stops:[...draft.stops].sort((a,b)=>a.offset-b.offset)}:null;
    const color=gradient?.stops[0]?.color??shape[target]??"#000000";
    onChange(target==="fill"?{fill:color,fillGradient:gradient,fillPattern:pattern}:{stroke:color,strokeGradient:gradient,strokePattern:pattern,strokeWidth:shape.strokeWidth||4});
  }
  return <details className="paint-editor">
    <summary>Peinture SVG : dégradés et motifs</summary>
    <fieldset disabled={disabled||converting}>
      <label>Cible<select aria-label="Cible du dégradé" value={target} onChange={e=>setTarget(e.target.value as typeof target)}><option value="fill">Fond</option><option value="stroke">Trait</option></select></label>
      <label>Peinture<select aria-label="Type de peinture" value={pattern?"pattern":geometry?.kind??"solid"} onChange={e=>{setPattern(e.target.value==="pattern"?initialPattern():null);setDraft(e.target.value==="solid"||e.target.value==="pattern"?null:initial(e.target.value as "linear"|"radial",shape[target]??"#000000"));}}><option value="solid">Couleur unie</option><option value="linear">Dégradé linéaire</option><option value="radial">Dégradé radial</option><option value="pattern">Motif vectoriel</option></select></label>
      {pattern&&<><label>Repère<select aria-label="Unités du motif" value={pattern.units} onChange={e=>patternUnits(e.target.value as SvgPattern["units"])}><option value="objectBoundingBox">Local à la forme</option><option value="userSpaceOnUse">Coordonnées du parent</option></select></label><PatternFields pattern={pattern} onChange={setPattern}/></>}
      {draft&&<>
        <label>Repère<select aria-label="Unités du dégradé" value={draft.units} onChange={e=>units(e.target.value as Gradient["units"])}><option value="objectBoundingBox">Local à la forme</option><option value="userSpaceOnUse">Coordonnées du parent</option></select></label>
        <label>Prolongement<select aria-label="Prolongement du dégradé" value={draft.spread} onChange={e=>update({spread:e.target.value as Gradient["spread"]})}><option value="pad">Couleurs des extrémités</option><option value="reflect">Miroir</option><option value="repeat">Répéter</option></select></label>
        <div className="paint-preview" aria-label="Couleurs du dégradé" style={{background:`linear-gradient(90deg,${draft.stops.map(s=>`${s.color}${Math.round(s.opacity*255).toString(16).padStart(2,"0")} ${s.offset*100}%`).join(",")})`}}/>
        <div className="field-grid">{coordinates.map(([label,value,change])=><label key={label}>{label}<input aria-label={`Dégradé ${label}`} type="number" step="any" value={value} onChange={e=>change(+e.target.value)}/></label>)}</div>
        <div className="paint-stops">{draft.stops.map((stop,i)=><div key={i}>
          <label>Position %<input aria-label={`Position couleur ${i+1}`} type="number" min="0" max="100" step="any" value={Math.round(stop.offset*10000)/100} onChange={e=>update({stops:draft.stops.map((s,j)=>j===i?{...s,offset:+e.target.value/100}:s)})}/></label>
          <input aria-label={`Couleur du dégradé ${i+1}`} type="color" value={stop.color} onChange={e=>update({stops:draft.stops.map((s,j)=>j===i?{...s,color:e.target.value}:s)})}/>
          <label>Opacité %<input aria-label={`Opacité couleur ${i+1}`} type="number" min="0" max="100" step="any" value={Math.round(stop.opacity*10000)/100} onChange={e=>update({stops:draft.stops.map((s,j)=>j===i?{...s,opacity:+e.target.value/100}:s)})}/></label>
          <button disabled={draft.stops.length<=2} aria-label={`Retirer couleur ${i+1}`} onClick={()=>update({stops:draft.stops.filter((_,j)=>j!==i)})}>−</button>
        </div>)}</div>
        <button disabled={draft.stops.length>=64} onClick={()=>update({stops:[...draft.stops,{offset:.5,color:"#808080",opacity:1}].sort((a,b)=>a.offset-b.offset)})}>Ajouter une couleur</button>
        <details><summary>Transformation du dégradé</summary><div className="field-grid">{draft.transform.map((v,i)=><label key={i}>{["A","B","C","D","X","Y"][i]}<input aria-label={`Matrice dégradé ${i+1}`} type="number" step="any" value={v} onChange={e=>update({transform:draft.transform.map((x,j)=>j===i?+e.target.value:x) as Gradient["transform"]})}/></label>)}</div><button onClick={()=>update({transform:[...identity]})}>Réinitialiser la transformation</button></details>
      </>}
      <div className="button-row"><button onClick={apply}>Appliquer la peinture</button><button onClick={reset}>Annuler les réglages</button></div>
      <p className="hint">{pattern?"Le motif est réglé en brouillon. ":"Les couleurs sont prévisualisées ci-dessus. "}Le document change uniquement avec Appliquer.</p>
      {error&&<p role="alert">{error}</p>}
    </fieldset>
  </details>;
}
