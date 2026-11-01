import { useState } from "react";
import type { Layer, Operation, Shape } from "./types";
import type { Point } from "./geometry";
import { absoluteContours, normalizePath } from "./paths";
import {siblings} from "./groups";
export default function PathPanel({
  shape,
  layer,
  action,
  nodes,
}: {
  shape: Shape;
  layer: Layer;
  action: (o: Operation) => Promise<unknown>;
  nodes: () => void;
}) {
  const [index, setIndex] = useState(0),
    [second, setSecond] = useState("");
  const cs = absoluteContours(shape),
    flat = cs.flatMap((c, ci) => c.nodes.map((n, ni) => ({ n, ci, ni })));
  const entry = flat[Math.min(index, flat.length - 1)];
  const save = () =>
    void action({
      type: "shape",
      id: layer.id,
      shape: normalizePath(shape, cs),
    });
  if (shape.type !== "path")
    return shape.type === "svg" ? (
      <div className="path-panel">
        <button
          onClick={() =>
            void action({
              type: "decomposeSvg",
              id: layer.id,
              shapeId: shape.id,
            })
          }
        >
          Décomposer le SVG en tracés
        </button>
        <p className="hint">
          Formes simples et texte en contours. Les traits deviennent des
          surfaces ; dégradés et motifs simples gardent leurs paramètres éditables.
          Les groupes avec opacité/fusion/isolation conservent leurs enfants.
          Images, motifs imbriqués, filtres, masques et découpes restent dans leur bloc original.
        </p>
      </div>
    ) : ["rect", "ellipse", "line", "text"].includes(shape.type) ? (
      <button
        onClick={() =>
          void action({ type: "toPath", id: layer.id, shapeId: shape.id })
        }
      >
        {shape.type==='text'?'Convertir le texte en contours':'Convertir en tracé'}
      </button>
    ) : null;
  const other =
    siblings(layer.shapes??[],shape.id).filter((s) => s.id !== shape.id && s.type === "path");
  const split = () => {
    if (!entry) return;
    const c = cs[entry.ci],
      a = c.nodes[entry.ni],
      b = c.nodes[(entry.ni + 1) % c.nodes.length];
    if (!c.closed && entry.ni === c.nodes.length - 1) return;
    const mid = (a: Point, b: Point): Point => [
      (a[0] + b[0]) / 2,
      (a[1] + b[1]) / 2,
    ];
    const ab = mid(a.point, a.outgoing),
      bc = mid(a.outgoing, b.incoming),
      cd = mid(b.incoming, b.point),
      abc = mid(ab, bc),
      bcd = mid(bc, cd),
      point = mid(abc, bcd);
    a.outgoing = ab;
    b.incoming = cd;
    c.nodes.splice(entry.ni + 1, 0, { point, incoming: abc, outgoing: bcd });
    setIndex(index + 1);
    save();
  };
  return (
    <div className="path-panel">
      <button onClick={nodes}>Éditer les nœuds à la souris</button>
      <label>
        Règle de remplissage{" "}
        <select
          aria-label="Règle de remplissage"
          value={shape.evenOdd ? "evenodd" : "nonzero"}
          onChange={(e) =>
            void action({
              type: "shape",
              id: layer.id,
              shape: { ...shape, evenOdd: e.target.value === "evenodd" },
            })
          }
        >
          <option value="nonzero">Non nul</option>
          <option value="evenodd">Pair-impair</option>
        </select>
      </label>
      {entry && (
        <>
          <label>
            Nœud{" "}
            <select
              aria-label="Nœud actif"
              value={Math.min(index, flat.length - 1)}
              onChange={(e) => setIndex(+e.target.value)}
            >
              {flat.map((e, i) => (
                <option key={i} value={i}>
                  Contour {e.ci + 1} · point {e.ni + 1}
                </option>
              ))}
            </select>
          </label>
          <label>
            <input
              type="checkbox"
              checked={cs[entry.ci].closed}
              onChange={(e) => {
                cs[entry.ci].closed = e.target.checked;
                save();
              }}
            />{" "}
            Contour fermé
          </label>
          <div className="button-row">
            <button
              onClick={() => {
                entry.n.incoming = [...entry.n.point];
                entry.n.outgoing = [...entry.n.point];
                save();
              }}
            >
              Angle
            </button>
            <button
              onClick={() => {
                const n = entry.n;
                let dx = n.outgoing[0] - n.point[0],
                  dy = n.outgoing[1] - n.point[1];
                if (Math.hypot(dx, dy) < 0.001) {
                  dx = 20;
                  dy = 0;
                }
                n.outgoing = [n.point[0] + dx, n.point[1] + dy];
                n.incoming = [n.point[0] - dx, n.point[1] - dy];
                save();
              }}
            >
              Lisser
            </button>
          </div>
          <button
            onClick={split}
            disabled={
              !cs[entry.ci].closed && entry.ni === cs[entry.ci].nodes.length - 1
            }
          >
            Scinder le segment suivant
          </button>
          <button
            disabled={cs[entry.ci].nodes.length <= 2}
            onClick={() => {
              cs[entry.ci].nodes.splice(entry.ni, 1);
              setIndex(Math.max(0, index - 1));
              save();
            }}
          >
            Supprimer le nœud
          </button>
        </>
      )}
      {other.length > 0 && (
        <>
          <label>
            Seconde forme{" "}
            <select
              aria-label="Seconde forme"
              value={second}
              onChange={(e) => setSecond(e.target.value)}
            >
              <option value="">Choisir un tracé</option>
              {other.map((s) => (
                <option key={s.id} value={s.id}>
                  Tracé #{s.id}
                </option>
              ))}
            </select>
          </label>
          <div className="boolean-buttons">
            {[
              ["union", "Union"],
              ["difference", "Différence"],
              ["intersection", "Intersection"],
              ["xor", "Exclusion"],
            ].map(([kind, label]) => (
              <button
                key={kind}
                disabled={!other.some((s) => s.id === +second)}
                onClick={() =>
                  void action({
                    type: "boolean",
                    id: layer.id,
                    first: shape.id,
                    second: +second,
                    kind,
                  })
                }
              >
                {label}
              </button>
            ))}
          </div>
          <p className="hint">
            Tracés fermés avec fond. Courbes approchées à 0,25 px ; le résultat
            prend le style de la première forme.
          </p>
        </>
      )}
    </div>
  );
}
