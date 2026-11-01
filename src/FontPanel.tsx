import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {open,save} from '@tauri-apps/plugin-dialog';
import type {Doc,FontInfo,FontOptions,Layer,Operation,Shape} from './types';
import {entries} from './groups';
import N from './DraftNumber';
const defaults=():FontOptions=>({axes:{},opticalSizing:'auto'});
let catalogPromise:Promise<string[]>|null=null;
const catalog=()=>catalogPromise??(catalogPromise=invoke<string[]>('font_catalog').catch(e=>{catalogPromise=null;throw e;}));
const axisNames:Record<string,string>={wght:'Graisse',opsz:'Taille optique',wdth:'Largeur',slnt:'Inclinaison',ital:'Italique'};
export default function FontPanel({shape,layer,doc,disabled,onChange,action}:{shape:Shape;layer:Layer;doc:Doc;disabled:boolean;onChange:(p:Partial<Shape>)=>void;action:(o:Operation)=>Promise<unknown>}){
 const [families,setFamilies]=useState<string[]>([]),[font,setFont]=useState<FontInfo|null>(null),[draft,setDraft]=useState<FontOptions>(()=>structuredClone(shape.fontOptions??defaults())),[result,setResult]=useState<{shape:Shape;src:string}|null>(null),[pending,setPending]=useState(false),[error,setError]=useState('');
 const [name,setName]=useState('Mon style'),[selected,setSelected]=useState(''),[all,setAll]=useState(false),[file,setFile]=useState('');
 const shapeSignature=JSON.stringify(shape),sourceSignature=JSON.stringify(layer.shapes),signature=JSON.stringify(draft);
 useEffect(()=>{let current=true;catalog().then(f=>{if(current)setFamilies(f);}).catch(e=>{if(current)setError(String(e));});return()=>{current=false;};},[]);
 useEffect(()=>{let current=true;setFont(null);invoke<FontInfo>('font_info',{family:shape.fontFamily??'',bold:shape.bold??false,italic:shape.italic??false}).then(f=>{if(current)setFont(f);}).catch(e=>{if(current)setError(String(e));});return()=>{current=false;};},[shape.fontFamily,shape.bold,shape.italic]);
 useEffect(()=>setDraft(structuredClone(shape.fontOptions??defaults())),[shapeSignature]);
 useEffect(()=>{let current=true;setPending(true);setResult(null);setError('');const timer=setTimeout(()=>invoke<{shape:Shape;src:string}>('font_preview',{shapes:layer.shapes,shape:{...shape,fontOptions:draft}}).then(r=>{if(current)setResult(r);}).catch(e=>{if(current)setError(String(e));}).finally(()=>{if(current)setPending(false);}),160);return()=>{current=false;clearTimeout(timer);};},[sourceSignature,signature]);
 useEffect(()=>{if(!doc.textStyles.some(s=>s.name===selected))setSelected(doc.textStyles[0]?.name??'');},[JSON.stringify(doc.textStyles)]);
 const axis=(tag:string,value:number)=>setDraft(d=>({...d,axes:{...d.axes,[tag]:value}}));
 const choose=async(mode:'open'|'save')=>{const options={filters:[{name:'Styles de texte Prisme',extensions:['prisme-text-styles']}]};const p=mode==='open'?await open({...options,multiple:false}):await save({...options,defaultPath:'styles.prisme-text-styles'});if(typeof p==='string')setFile(p);};
 return <section className="font-panel">
  {font&&!font.available&&<p role="alert" className="font-missing">Police absente : « {font.requested} ». Aperçu avec « {font.resolved??'aucune police disponible'} ». Choisissez un remplacement ci-dessous ; le nom demandé reste conservé jusqu’à votre choix.</p>}
  <details><summary>Police variable et styles</summary><fieldset disabled={disabled}>
   <label>Polices installées<select aria-label="Polices disponibles" value={families.includes(shape.fontFamily??'')?shape.fontFamily:''} onChange={e=>{if(e.target.value)onChange({fontFamily:e.target.value,fontOptions:null});}}><option value="">Choisir une police installée…</option>{families.map(f=><option key={f} value={f}>{f}</option>)}</select></label>
   {font?.available&&<p className="hint">{font.axes.length?`Police variable : ${font.axes.length} axes disponibles.`:'Police à dessin fixe : aucun axe variable.'} Les polices sont recensées au démarrage de l’application.</p>}
   {result&&<div className="filter-preview"><img alt="Aperçu des variations de police" src={result.src}/></div>}{pending&&<p role="status">Calcul de l’aperçu typographique…</p>}
   {font?.axes.map(a=>{const automatic=a.tag==='opsz'&&draft.opticalSizing==='auto'?shape.fontSize??a.default:a.tag==='wght'?(shape.bold?700:400):a.tag==='ital'&&shape.italic?1:a.default;const value=draft.axes[a.tag]??Math.max(a.min,Math.min(a.max,automatic));return <section className="font-axis" key={a.tag}><h3>{axisNames[a.tag]??a.name} · {a.tag}</h3><p className="hint">De {a.min} à {a.max} · valeur de la police {a.default}{a.hidden?' · axe masqué par la police':''}</p><N label={`Axe ${a.tag}`} value={value} onChange={v=>axis(a.tag,v)}/><input type="range" aria-label={`Curseur axe ${a.tag}`} min={a.min} max={a.max} step={(a.max-a.min)/1000||1} value={value} onChange={e=>axis(a.tag,+e.target.value)}/><button disabled={draft.axes[a.tag]===undefined} onClick={()=>setDraft(d=>{const next=structuredClone(d);delete next.axes[a.tag];return next;})}>Libérer l’axe {a.tag}</button></section>;})}
   {!!font?.axes.some(a=>a.tag==='opsz')&&<label>Taille optique<select aria-label="Taille optique de la police" value={draft.opticalSizing} onChange={e=>setDraft(d=>({...d,opticalSizing:e.target.value as FontOptions['opticalSizing']}))}><option value="auto">Automatique selon le corps</option><option value="none">Valeur par défaut de la police</option></select></label>}
   <p className="hint">Une valeur saisie devient explicite et prime sur le réglage automatique correspondant, y compris Gras. Libérer un axe rétablit ce réglage. Les axes restent en brouillon jusqu’à Appliquer ; le contenu et les liens de tracé sont conservés.</p>
   <div className="button-row"><button disabled={pending||!result} onClick={()=>result&&onChange(result.shape)}>Appliquer les variations</button><button onClick={()=>setDraft(structuredClone(shape.fontOptions??defaults()))}>Annuler les variations</button><button onClick={()=>setDraft(defaults())}>Réinitialiser les axes</button></div>
   {error&&<p role="alert">{error}</p>}
   <section className="text-styles"><h3>Styles de texte du document</h3>
    <label>Nom du style<input aria-label="Nom du style de texte" value={name} onChange={e=>setName(e.target.value)}/></label><button disabled={!name.trim()} onClick={()=>void action({type:'textStyleSave',id:layer.id,shapeId:shape.id,name})}>Enregistrer le style du texte</button>
    <label>Style<select aria-label="Style de texte" value={selected} onChange={e=>{setSelected(e.target.value);setName(e.target.value);}}><option value="">Choisir un style…</option>{doc.textStyles.map(s=><option key={s.name} value={s.name}>{s.name}</option>)}</select></label>
    <label><input type="checkbox" aria-label="Style sur tous les textes du calque" checked={all} onChange={e=>setAll(e.target.checked)}/>Appliquer à tous les textes du calque</label>
    <div className="button-row"><button disabled={!selected} onClick={()=>void action({type:'textStyleApply',id:layer.id,shapeIds:all?entries(layer.shapes??[]).filter(e=>e.shape.type==='text').map(e=>e.shape.id):[shape.id],name:selected})}>Appliquer le style de texte</button><button disabled={!selected} onClick={()=>void action({type:'textStyleDelete',name:selected})}>Supprimer le style de texte</button></div>
    <p className="hint">Un nom déjà présent met le style à jour. Les styles copient la typographie et les peintures ; ils ne contiennent ni texte, ni position, ni chemin. Modifier ou supprimer un style ne change pas les textes auxquels il a déjà été appliqué.</p>
    <label>Bibliothèque de styles<input aria-label="Fichier des styles de texte" value={file} onChange={e=>setFile(e.target.value)}/></label><div className="button-row"><button onClick={()=>void choose('open')}>Choisir une bibliothèque…</button><button disabled={!file.trim()} onClick={()=>void action({type:'textStylesImport',path:file})}>Importer les styles de texte</button><button onClick={()=>void choose('save')}>Choisir la destination des styles…</button><button disabled={!file.trim()} onClick={()=>void action({type:'textStylesExport',path:file})}>Exporter les styles de texte</button></div>
    <p className="hint">Les noms déjà utilisés sont renommés à l’import. La bibliothèque contient les réglages ; les fichiers de police ne sont pas intégrés.</p>
   </section>
  </fieldset></details>
 </section>;
}
