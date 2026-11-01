import {useEffect,useRef,useState} from 'react';
import type {KeyboardEvent} from 'react';
import {invoke} from '@tauri-apps/api/core';
import type {Doc,Layer,Operation} from './types';
import N from './DraftNumber';
type Settings={mode:'mono'|'color';threshold:number;invert:boolean;color:string;colors:number;alphaThreshold:number;maxSide:number;simplify:number;smoothing:number;minArea:number;useSelection:boolean};
type Preview={revision:number;source:string;src:string;key:string;stats:{sourceWidth:number;sourceHeight:number;width:number;height:number;nodes:number;contours:number;paths:number;removedContours:number;palette:number[][];elapsedMs:number}};
const defaults=():Settings=>({mode:'mono',threshold:128,invert:false,color:'#000000',colors:8,alphaThreshold:1,maxSide:512,simplify:.5,smoothing:0,minArea:0,useSelection:false});
export default function VectorizeDialog({doc,layer,busy,onClose,action}:{doc:Doc;layer?:Layer;busy:boolean;onClose:()=>void;action:(o:Operation)=>Promise<Doc|undefined>}){
 const [settings,setSettings]=useState(defaults),[hide,setHide]=useState(true),[result,setResult]=useState<Preview|null>(null),[pending,setPending]=useState(false),[error,setError]=useState('');
 const box=useRef<HTMLElement>(null),key=JSON.stringify([layer?.id,doc.revision,settings]);
 useEffect(()=>{let current=true;setResult(null);setPending(true);setError('');const timer=setTimeout(()=>invoke<Omit<Preview,'key'>>('vectorize_preview',{layerId:layer?.id,revision:doc.revision,settings}).then(r=>{if(current)setResult({...r,key});}).catch(e=>{if(current)setError(String(e));}).finally(()=>{if(current)setPending(false);}),220);return()=>{current=false;clearTimeout(timer);};},[key]);
 const change=(p:Partial<Settings>)=>setSettings(s=>({...s,...p}));
 const number=(label:string,name:keyof Settings,integer=false)=><N label={label} value={settings[name] as number} onChange={v=>change({[name]:integer?Math.round(v):v})}/>;
 const canApply=!busy&&!pending&&result?.key===key;
 const keys=(e:KeyboardEvent<HTMLDivElement>)=>{
  if(e.key==='Escape'){e.stopPropagation();if(!busy)onClose();}
  if(e.key==='Tab'){
   const elements=box.current?.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled)');
   if(elements?.length){const first=elements[0],last=elements[elements.length-1];
    if(e.shiftKey&&document.activeElement===first){e.preventDefault();last.focus();}
    else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first.focus();}
   }
  }
 };
 return <div className="modal-backdrop" onPointerDown={e=>{if(e.target===e.currentTarget&&!busy)onClose();}} onKeyDown={keys}>
  <section className="modal vectorize-dialog" role="dialog" aria-modal="true" aria-labelledby="vectorize-title" ref={box}>
   <h2 id="vectorize-title">Vectoriser le bitmap</h2><p className="hint">Transformer « {layer?.name??'calque indisponible'} » en tracés éditables. Le bitmap reste dans le document. L’aperçu compare les pixels échantillonnés au résultat vectoriel.</p>
   <div className="vectorize-layout"><fieldset disabled={busy}><legend>Réglages</legend>
    <label>Mode<select autoFocus aria-label="Mode de vectorisation" value={settings.mode} onChange={e=>change({mode:e.target.value as Settings['mode']})}><option value="mono">Monochrome</option><option value="color">Couleur</option></select></label>
    {settings.mode==='mono'?<>{number('Seuil de luminosité','threshold',true)}<p className="hint">De 0 à 255 : garder les pixels plus sombres que le seuil.</p><label><input aria-label="Inverser le seuil" type="checkbox" checked={settings.invert} onChange={e=>change({invert:e.target.checked})}/>Garder les pixels plus clairs</label><label>Couleur des tracés<input aria-label="Couleur monochrome" type="color" value={settings.color} onChange={e=>change({color:e.target.value})}/></label></>:<>{number('Couleurs de la palette','colors',true)}<p className="hint">De 2 à 32 entrées RGBA, sans tramage. Les nuances et la transparence sont regroupées dans cette palette.</p></>}
    {number('Seuil de transparence','alphaThreshold',true)}<p className="hint">De 1 à 255. Les pixels d’alpha inférieur sont ignorés. Le monochrome conserve 15 niveaux d’opacité.</p>
    {number('Résolution maximale','maxSide',true)}<p className="hint">De 32 à 1024 pixels sur le grand côté. Les tracés gardent les dimensions du bitmap ; une résolution réduite perd les petits détails.</p>
    {number('Simplification en pixels','simplify')}{number('Arrondi des angles','smoothing')}{number('Petites zones en pixels carrés','minArea')}<p className="hint">Simplification 0–4 px, arrondi 0–1, petites zones 0–4096 px². Zéro conserve les contours des pixels. Les petites îles et les petits trous peuvent être retirés.</p>
    <label><input aria-label="Limiter la vectorisation à la sélection" type="checkbox" checked={settings.useSelection} disabled={!doc.selection} onChange={e=>change({useSelection:e.target.checked})}/>Limiter à la sélection actuelle</label>
    <label><input aria-label="Masquer le bitmap après vectorisation" type="checkbox" checked={hide} onChange={e=>setHide(e.target.checked)}/>Masquer le bitmap après application</label>
    <p className="hint">Si la sélection est utilisée, seul son contenu est vectorisé. Masquer le bitmap masque aussi sa partie hors sélection. Opacité, fusion, effets et position du calque sont repris sur le nouveau calque.</p>
    <button type="button" onClick={()=>setSettings(defaults())}>Réinitialiser la vectorisation</button>
   </fieldset><div className="vectorize-results" aria-live="polite">
    {pending&&<p role="status">Calcul de la vectorisation…</p>}{error&&<p role="alert">{error}</p>}
    {result?.key===key&&<><figure><figcaption>Pixels échantillonnés</figcaption><div className="vectorize-preview"><img src={result.source} alt="Bitmap avant vectorisation"/></div></figure><figure><figcaption>Tracés proposés</figcaption><div className="vectorize-preview"><img src={result.src} alt="Aperçu de la vectorisation"/></div></figure><p className="vectorize-stats">{result.stats.paths} tracé(s), {result.stats.contours} contour(s), {result.stats.nodes} nœuds · grille {result.stats.width} × {result.stats.height} · {result.stats.elapsedMs} ms</p><p className="hint">{result.stats.removedContours} petite(s) zone(s) retirée(s). Les couleurs ci-dessous sont les entrées utilisées, transparence comprise.</p><div className="vectorize-palette" aria-label="Palette de vectorisation">{result.stats.palette.map((c,i)=><span key={i} title={`RGBA ${c.join(', ')}`} style={{backgroundColor:`rgba(${c[0]},${c[1]},${c[2]},${c[3]/255})`}}/>)}</div></>}
    <p className="hint">Ce traitement convient aux logos, dessins et aplats. Il ne reconstitue pas les objets d’une photographie. Les contours et leurs nœuds deviennent modifiables dans le panneau des tracés. Une image trop complexe est refusée sans résultat partiel.</p>
   </div></div>
   <div className="modal-actions"><button type="button" disabled={busy} onClick={onClose}>Annuler la vectorisation</button><button type="button" className="primary" disabled={!canApply} onClick={()=>{if(result?.key===key)void action({type:'vectorize',id:layer?.id,revision:result.revision,settings,hideSource:hide}).then(next=>{if(next)onClose();});}}>Créer le calque vectoriel</button></div>
  </section>
 </div>;
}
