import {useEffect,useRef,useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import {open} from "@tauri-apps/plugin-dialog";
import type {Shape,ImageAsset,Operation} from "./types";
export default function ImagePanel({shape,disabled,onChange,action,onPendingChange,draft=false}:{shape:Shape;disabled:boolean;onChange:(p:Partial<Shape>)=>void;action:(o:Operation)=>Promise<unknown>;onPendingChange?:(p:boolean)=>void;draft?:boolean}){
  const asset=shape.asset!;
  const [path,setPath]=useState(""),[error,setError]=useState(""),[pending,setPending]=useState(false),[preview,setPreview]=useState("");
  const alive=useRef(true);useEffect(()=>{alive.current=true;return()=>{alive.current=false;};},[]);
  useEffect(()=>{onPendingChange?.(pending);return()=>onPendingChange?.(false);},[pending,onPendingChange]);
  const signature=JSON.stringify(asset);
  useEffect(()=>{let current=true;setPreview("");invoke<string>("embedded_image_preview",{asset}).then(src=>{if(current)setPreview(src);}).catch(e=>{if(current)setError(String(e));});return()=>{current=false;};},[signature]);
  async function replace(){setPending(true);setError("");try{const next=await invoke<ImageAsset>("embedded_image_file",{path});if(alive.current)onChange({asset:{...next,rendering:asset.rendering}});}catch(e){if(alive.current)setError(String(e));}finally{if(alive.current)setPending(false);}}
  return <section className="image-panel"><h3>Image intégrée</h3>
    {preview&&<div className="embedded-preview"><img alt="Aperçu de l’image intégrée" src={preview}/></div>}
    <p className="hint">{asset.mime.replace('image/','').toUpperCase()} · {asset.width} × {asset.height} · données intégrées au projet. {asset.mime==='image/gif'?'La première image du GIF est utilisée.':''}</p>
    <fieldset disabled={disabled||pending}><label>Interpolation<select aria-label="Interpolation de l’image" value={asset.rendering} onChange={e=>onChange({asset:{...asset,rendering:e.target.value}})}>{[['optimizeQuality','Qualité'],['optimizeSpeed','Voisin le plus proche'],['smooth','Lissé'],['high-quality','Haute qualité'],['crisp-edges','Bords nets'],['pixelated','Pixel art']].map(([v,l])=><option key={v} value={v}>{l}</option>)}</select></label>
      <label>Fichier de remplacement<input aria-label="Fichier de remplacement" value={path} onChange={e=>setPath(e.target.value)}/></label>
      <div className="button-row"><button onClick={async()=>{const value=await open({multiple:false,filters:[{name:'Image intégrée',extensions:['png','jpg','jpeg','gif','webp','svg']}]});if(typeof value==='string')setPath(value);}}>Parcourir…</button><button disabled={!path.trim()} onClick={()=>void replace()}>Remplacer l’image</button></div>
      <p className="hint">Le remplacement garde la position, les dimensions, les transformations et l’opacité. {draft?'Cette modification reste dans le brouillon ; appliquez ensuite les effets.':'Une seule annulation retrouve les données précédentes.'}</p>
      {asset.mime==='image/svg+xml'&&<button onClick={()=>void action({type:'decomposeSvg',shapeId:shape.id})}>Décomposer l’image SVG interne</button>}
    </fieldset>{error&&<p role="alert">{error}</p>}
  </section>;
}
