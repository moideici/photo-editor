import { useState } from "react";
import type { Layer, Shape, Doc, Operation } from "./types";
import { siblings, context, entries } from "./groups";

export default function GroupPanel({ layer, shape, busy, action, select }: {
  layer: Layer; shape?: Shape; busy: boolean;
  action: (o: Operation) => Promise<Doc | undefined>;
  select: (id: number | null) => void;
}) {
  const [chosen, setChosen] = useState<number[]>([]);
  const all = layer.shapes ?? [];
  const parentId = shape ? context(all, shape.id)?.parentId : null;
  const peers = shape ? siblings(all, shape.id) : all;
  const selected = chosen.filter(id => peers.some(s => s.id === id));
  const style=shape?.groupStyle??{blend:"normal",isolate:false};
  function changeStyle(patch:Partial<typeof style>){if(shape)void action({type:"shape",id:layer.id,shape:{...shape,groupStyle:{...style,...patch}}});}
  async function group() {
    const known = new Set(entries(all).map(e => e.shape.id));
    const result = await action({ type: "groupShapes", id: layer.id, shapeIds: selected });
    if (!result) return;
    const list = result.layers.find(l => l.id === layer.id)?.shapes ?? [];
    select(entries(list).find(e => !known.has(e.shape.id))?.shape.id ?? null);
    setChosen([]);
  }
  async function ungroup() {
    if (!shape) return;
    const result = await action({ type: "ungroupShapes", id: layer.id, shapeId: shape.id });
    if (result) select(shape.children?.[0]?.id ?? null);
  }
  return <div className="group-panel">
    <h3>Groupes vectoriels</h3>
    <p className="hint">Cochez des formes consécutives du même parent. Clic : groupe ; Alt+clic : enfant. Les coordonnées d’un enfant sont locales à son parent.</p>
    <div className="group-choices">{peers.map(s => <label key={s.id}>
      <input type="checkbox" aria-label={`Grouper ${s.type} #${s.id}`} disabled={busy}
        checked={selected.includes(s.id)} onChange={e => setChosen(e.target.checked ? [...selected, s.id] : selected.filter(id => id !== s.id))} />
      {s.type === "group" ? "Groupe" : s.type === "text" ? s.text : s.type} #{s.id}
    </label>)}</div>
    <button disabled={busy || selected.length < 2} onClick={() => void group()}>Grouper les formes cochées</button>
    {shape?.type === "group" && <>
      <label className="group-style-field">Fusion du groupe<select aria-label="Fusion du groupe" disabled={busy} value={style.blend} onChange={e=>changeStyle({blend:e.target.value})}>
        {[["normal","Normal"],["multiply","Produit"],["screen","Superposition"],["overlay","Incrustation"],["darken","Obscurcir"],["lighten","Éclaircir"],["color-dodge","Densité couleur −"],["color-burn","Densité couleur +"],["hard-light","Lumière crue"],["soft-light","Lumière tamisée"],["difference","Différence"],["exclusion","Exclusion"],["hue","Teinte"],["saturation","Saturation"],["color","Couleur"],["luminosity","Luminosité"],["plus-lighter","Lumière additive"]].map(([value,label])=><option key={value} value={value}>{label}</option>)}
      </select></label>
      <label className="group-isolation"><input aria-label="Isoler la fusion des enfants" type="checkbox" disabled={busy} checked={style.isolate} onChange={e=>changeStyle({isolate:e.target.checked})}/>Isoler la fusion des enfants</label>
      <p className="hint">L’opacité et la fusion portent sur l’ensemble. L’isolation limite la fusion des enfants au contenu du groupe ; les enfants restent éditables.</p>
      <button disabled={busy} onClick={() => void ungroup()}>Dissocier le groupe</button>
      <button disabled={busy} onClick={() => select(shape.children?.[0]?.id ?? null)}>Sélectionner le premier enfant</button>
      {shape.opacity !== 1 && <p className="hint">Dissociation sans perte : remettez l’opacité du groupe à 100 % avant de dissocier. Les opacités des enfants restent conservées.</p>}
      {(style.blend!=="normal"||style.isolate)&&<p className="hint">Pour dissocier sans changer le rendu, choisissez Normal et désactivez l’isolation du groupe. Les styles des enfants restent conservés.</p>}
    </>}
    {parentId && <button disabled={busy} onClick={() => select(parentId)}>Sélectionner le groupe parent</button>}
  </div>;
}
