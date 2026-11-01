import { useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
type Point = [number, number];
export type CurvesData = Record<"rgb" | "red" | "green" | "blue", Point[]>;
type Channel = keyof CurvesData;
type Histogram = Record<"red" | "green" | "blue" | "luma", number[]>;
export const identityCurves = (): CurvesData => ({
  rgb: [
    [0, 0],
    [255, 255],
  ],
  red: [
    [0, 0],
    [255, 255],
  ],
  green: [
    [0, 0],
    [255, 255],
  ],
  blue: [
    [0, 0],
    [255, 255],
  ],
});
function samples(points: Point[]): number[] {
  const n = points.length,
    dx = points.slice(1).map((p, i) => p[0] - points[i][0]),
    d = points.slice(1).map((p, i) => (p[1] - points[i][1]) / dx[i]),
    slope = Array(n).fill(0) as number[];
  slope[0] = d[0];
  slope[n - 1] = d[n - 2];
  for (let i = 1; i < n - 1; i++)
    if (d[i - 1] * d[i] > 0) {
      const a = 2 * dx[i] + dx[i - 1],
        b = dx[i] + 2 * dx[i - 1];
      slope[i] = (a + b) / (a / d[i - 1] + b / d[i]);
    }
  let i = 0;
  return Array.from({ length: 256 }, (_, x) => {
    if (x <= points[0][0]) return points[0][1];
    if (x >= points[n - 1][0]) return points[n - 1][1];
    while (i + 1 < n - 1 && x > points[i + 1][0]) i++;
    const t = (x - points[i][0]) / dx[i],
      t2 = t * t,
      t3 = t2 * t;
    return Math.max(
      0,
      Math.min(
        255,
        (2 * t3 - 3 * t2 + 1) * points[i][1] +
          (t3 - 2 * t2 + t) * dx[i] * slope[i] +
          (-2 * t3 + 3 * t2) * points[i + 1][1] +
          (t3 - t2) * dx[i] * slope[i + 1],
      ),
    );
  });
}
export default function CurvesEditor({
  value,
  onChange,
  layerId,
}: {
  value: CurvesData;
  onChange: (v: CurvesData) => void;
  layerId: number;
}) {
  const [channel, setChannel] = useState<Channel>("rgb"),
    [selected, setSelected] = useState(0),
    [histogram, setHistogram] = useState<Histogram | null>(null),
    [error, setError] = useState("");
  const grab = useRef<number | null>(null),
    svg = useRef<SVGSVGElement>(null),
    points = value[channel];
  useEffect(() => {
    let cancelled = false;
    if (isTauri())
      void invoke<Histogram>("layer_histogram", { id: layerId })
        .then((h) => {
          if (!cancelled) setHistogram(h);
        })
        .catch((e) => {
          if (!cancelled) setError(String(e));
        });
    return () => {
      cancelled = true;
    };
  }, [layerId]);
  const update = (i: number, x: number, y: number) => {
    const next = points.map((p) => [...p] as Point);
    next[i] = [
      i === 0
        ? 0
        : i === points.length - 1
          ? 255
          : Math.max(
              points[i - 1][0] + 1,
              Math.min(points[i + 1][0] - 1, Math.round(x)),
            ),
      Math.max(0, Math.min(255, Math.round(y))),
    ];
    onChange({ ...value, [channel]: next });
  };
  const remove = (i: number) => {
    if (i === 0 || i === points.length - 1) return;
    onChange({ ...value, [channel]: points.filter((_, j) => i !== j) });
    setSelected(Math.max(0, i - 1));
  };
  const coordinates = (clientX: number, clientY: number): Point => {
    const r = svg.current!.getBoundingClientRect();
    return [
      Math.max(
        0,
        Math.min(255, Math.round(((clientX - r.left) / r.width) * 255)),
      ),
      Math.max(
        0,
        Math.min(255, Math.round(255 - ((clientY - r.top) / r.height) * 255)),
      ),
    ];
  };
  const hist =
      histogram?.[channel === "rgb" ? "luma" : channel] ?? Array(256).fill(0),
    max = Math.max(1, ...hist),
    curve = samples(points),
    node = points[selected] ?? points[0];
  return (
    <div className="curves-editor">
      <label>
        Canal
        <select
          aria-label="Canal des courbes"
          value={channel}
          onChange={(e) => {
            setChannel(e.target.value as Channel);
            setSelected(0);
            grab.current = null;
          }}
        >
          <option value="rgb">RVB</option>
          <option value="red">Rouge</option>
          <option value="green">Vert</option>
          <option value="blue">Bleu</option>
        </select>
      </label>
      <svg
        ref={svg}
        className="curve-graph"
        viewBox="0 0 255 255"
        preserveAspectRatio="none"
        role="img"
        aria-label="Courbe tonale"
        tabIndex={0}
        onPointerDown={(e) => {
          if (e.button !== 0 || points.length >= 32) return;
          const p = coordinates(e.clientX, e.clientY);
          if (
            p[0] === 0 ||
            p[0] === 255 ||
            points.some((v) => Math.abs(v[0] - p[0]) < 2)
          )
            return;
          const next = [...points, p].sort((a, b) => a[0] - b[0]),
            i = next.indexOf(p);
          onChange({ ...value, [channel]: next });
          setSelected(i);
          grab.current = i;
          e.currentTarget.setPointerCapture(e.pointerId);
        }}
        onPointerMove={(e) => {
          if (grab.current === null) return;
          const p = coordinates(e.clientX, e.clientY);
          update(grab.current, ...p);
        }}
        onPointerUp={() => {
          grab.current = null;
        }}
        onPointerCancel={() => {
          grab.current = null;
        }}
        onKeyDown={(e) => {
          if (e.key === "Delete" || e.key === "Backspace") {
            e.preventDefault();
            remove(selected);
          } else if (e.key.startsWith("Arrow")) {
            e.preventDefault();
            update(
              selected,
              node[0] +
                (e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0),
              node[1] +
                (e.key === "ArrowUp" ? 1 : e.key === "ArrowDown" ? -1 : 0),
            );
          }
        }}
      >
        <path
          className="histogram"
          d={`M0 255 ${hist.map((v, i) => `L${i} ${255 - (Math.log1p(v) / Math.log1p(max)) * 220}`).join(" ")} L255 255Z`}
        />
        {[64, 128, 192].map((n) => (
          <path key={n} className="curve-grid" d={`M${n} 0V255 M0 ${n}H255`} />
        ))}
        <path className="curve-reference" d="M0 255L255 0" />
        <path
          className="curve-line"
          d={curve.map((y, x) => `${x ? "L" : "M"}${x} ${255 - y}`).join(" ")}
        />
        {points.map((point, i) => (
          <circle
            key={i}
            className={i === selected ? "curve-node selected" : "curve-node"}
            cx={point[0]}
            cy={255 - point[1]}
            r={3.5}
            onPointerDown={(e) => {
              e.stopPropagation();
              if (e.button !== 0) return;
              setSelected(i);
              grab.current = i;
              svg.current!.setPointerCapture(e.pointerId);
            }}
            onContextMenu={(e) => {
              e.preventDefault();
              remove(i);
            }}
          />
        ))}
      </svg>
      <div className="field-grid">
        <label>
          Entrée
          <input
            aria-label="Entrée du point de courbe"
            type="number"
            min={selected === 0 ? 0 : (points[selected - 1]?.[0] ?? -1) + 1}
            max={
              selected === points.length - 1
                ? 255
                : (points[selected + 1]?.[0] ?? 256) - 1
            }
            readOnly={selected === 0 || selected === points.length - 1}
            value={node[0]}
            onChange={(e) => update(selected, +e.target.value, node[1])}
          />
        </label>
        <label>
          Sortie
          <input
            aria-label="Sortie du point de courbe"
            type="number"
            min={0}
            max={255}
            value={node[1]}
            onChange={(e) => update(selected, node[0], +e.target.value)}
          />
        </label>
      </div>
      <div className="curve-actions">
        <button
          type="button"
          onClick={() => {
            onChange({
              ...value,
              [channel]: [
                [0, 0],
                [255, 255],
              ],
            });
            setSelected(0);
          }}
        >
          Réinitialiser le canal
        </button>
        <button
          type="button"
          disabled={selected === 0 || selected === points.length - 1}
          onClick={() => remove(selected)}
        >
          Supprimer le point
        </button>
      </div>
      <p className="hint">
        Cliquer pour ajouter, glisser pour déplacer. Flèches : ajuster. Suppr ou
        clic droit : retirer un point.
      </p>
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
