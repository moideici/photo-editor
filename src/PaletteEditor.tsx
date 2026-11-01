import { useEffect, useState } from "react";
import type { IndexedInfo } from "./types";
export const hexColor = (p: number[], alpha = false) =>
  "#" +
  p
    .slice(0, alpha ? 4 : 3)
    .map((v) => v.toString(16).padStart(2, "0"))
    .join("");
export default function PaletteEditor({
  value,
  onChange,
  onUse,
  onRgb,
  disabled,
}: {
  value: IndexedInfo;
  onChange: (index: number, color: string) => void;
  onUse: (index: number) => void;
  onRgb: () => void;
  disabled: boolean;
}) {
  const [selected, setSelected] = useState(0),
    [color, setColor] = useState("#000000"),
    [alpha, setAlpha] = useState(255);
  const index = Math.min(selected, value.palette.length - 1),
    entry = value.palette[index],
    entryKey = entry.join(",");
  useEffect(() => {
    setColor(hexColor(entry));
    setAlpha(entry[3]);
  }, [index, entryKey]);
  return (
    <section className="panel palette-panel">
      <h2>
        Palette indexée <small>{value.palette.length} couleurs</small>
      </h2>
      <div className="palette-grid">
        {value.palette.map((p, i) => (
          <button
            type="button"
            className={`palette-swatch checker ${i === index ? "selected" : ""}`}
            aria-label={`Couleur indexée ${i}`}
            title={`${i} · ${hexColor(p, true)} · ${value.counts[i]} pixels`}
            key={i}
            disabled={disabled}
            onClick={() => {
              setSelected(i);
              onUse(i);
            }}
          >
            <span
              style={{
                background: `rgba(${p[0]},${p[1]},${p[2]},${p[3] / 255})`,
              }}
            />
          </button>
        ))}
      </div>
      <p className="hint">
        Entrée {index} · {value.counts[index]} pixels
      </p>
      <div className="field-grid">
        <label>
          Couleur
          <input
            aria-label="Couleur de palette"
            type="color"
            value={color}
            onChange={(e) => setColor(e.target.value)}
            disabled={disabled}
          />
        </label>
        <label>
          Alpha
          <input
            aria-label="Alpha de palette"
            type="number"
            min={0}
            max={255}
            value={alpha}
            onChange={(e) =>
              setAlpha(Math.max(0, Math.min(255, +e.target.value)))
            }
            disabled={disabled}
          />
        </label>
      </div>
      <div className="palette-actions">
        <button
          type="button"
          disabled={
            disabled || (color === hexColor(entry) && alpha === entry[3])
          }
          onClick={() =>
            onChange(index, color + alpha.toString(16).padStart(2, "0"))
          }
        >
          Appliquer à la palette
        </button>
        <button type="button" disabled={disabled} onClick={onRgb}>
          Passer en RVB
        </button>
      </div>
      <p className="hint">
        Alpha 0 rend une entrée transparente. Les traits restent limités à cette
        palette. Les calques et les effets nécessitent le mode RVB.
      </p>
    </section>
  );
}
